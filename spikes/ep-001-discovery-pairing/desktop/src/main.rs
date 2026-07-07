// EP-002.5: Desktop Packaging
// - System tray with menu (Status, Open Logs, Exit)
// - File logging to %APPDATA%/AutoMatDeck/agent.log
// - Windows subsystem (no console window in release)
// - CLI: --install (auto-start), --uninstall (remove auto-start)
// - Single-instance via named mutex

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod actions;
mod discovery;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use discovery::AdvertisementProvider;
use futures_util::{SinkExt, StreamExt};
use log::{info, warn, error};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

use tray_icon::menu::{Menu, MenuItem, CheckMenuItem, PredefinedMenuItem, MenuEvent};
use tray_icon::{TrayIconBuilder, TrayIcon, Icon};

#[cfg(windows)]
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE};
#[cfg(windows)]
use windows_sys::Win32::System::Threading::CreateMutexW;
#[cfg(windows)]
use windows_sys::Win32::System::Registry::{
    RegOpenKeyExW, RegSetValueExW, RegDeleteValueW, RegCloseKey, RegQueryValueExW,
    HKEY_CURRENT_USER, KEY_SET_VALUE, KEY_QUERY_VALUE, REG_SZ,
};
#[cfg(windows)]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    PeekMessageW, TranslateMessage, DispatchMessageW, MSG, PM_REMOVE,
};

const PORT: u16 = 9742;
const PAIR_TIMEOUT_SECS: u64 = 120;
#[cfg(windows)]
const MUTEX_NAME: &str = "Local\\AutoMatDeck_Agent";
#[cfg(windows)]
const REGISTRY_PATH: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
#[cfg(windows)]
const REGISTRY_VALUE_NAME: &str = "AutoMatDeck Agent";

static ACTIONS: LazyLock<actions::ActionRegistry> = LazyLock::new(|| actions::ActionRegistry::new());

// --- Shared state for tray-based pairing ---

struct PendingPair {
    device_id: String,
    device_name: String,
    responder: tokio::sync::oneshot::Sender<bool>,
}

type AppState = Arc<Mutex<Option<PendingPair>>>;

struct MenuItems {
    tray: TrayIcon,
    status: MenuItem,
    start_on_login: CheckMenuItem,
    logs: MenuItem,
    exit: MenuItem,
}

// --- Trusted device store ---

#[derive(Serialize, Deserialize, Clone)]
struct TrustedDevice {
    device_id: String,
    device_name: String,
    last_seen: u64,
    paired_at: u64,
}

fn get_data_dir() -> PathBuf {
    std::env::var("APPDATA")
        .map(|p| PathBuf::from(p).join("AutoMatDeck"))
        .unwrap_or_else(|_| PathBuf::from("data"))
}

fn store_path() -> PathBuf {
    get_data_dir().join("trusted_devices.json")
}

fn load_devices() -> Vec<TrustedDevice> {
    let path = store_path();
    if path.exists() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        Vec::new()
    }
}

fn save_devices(devices: &[TrustedDevice]) {
    let path = store_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    if let Ok(content) = serde_json::to_string_pretty(devices) {
        std::fs::write(&path, &content).ok();
    }
}

fn is_trusted(device_id: &str) -> bool {
    load_devices().iter().any(|d| d.device_id == device_id)
}

fn add_device(device_id: &str, device_name: &str) {
    let mut devices = load_devices();
    devices.retain(|d| d.device_id != device_id);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    devices.push(TrustedDevice {
        device_id: device_id.to_string(),
        device_name: device_name.to_string(),
        last_seen: now,
        paired_at: now,
    });
    save_devices(&devices);
}

fn touch_device(device_id: &str) {
    let mut devices = load_devices();
    if let Some(d) = devices.iter_mut().find(|d| d.device_id == device_id) {
        d.last_seen = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        save_devices(&devices);
    }
}

// --- WebSocket handler (pairing via tray approval) ---

async fn handle_connection(
    stream: tokio::net::TcpStream,
    peer: SocketAddr,
    app_state: AppState,
) {
    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            warn!("WebSocket handshake failed from {}: {}", peer, e);
            return;
        }
    };

    let (mut write, mut read) = ws_stream.split();
    let mut trusted = false;
    let mut device_id: Option<String> = None;

    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => match serde_json::from_str::<Value>(&text) {
                Ok(req) => {
                    let msg_type = req
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");

                    match msg_type {
                        "identify" => {
                            let id = req
                                .get("device_id")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            let name = req
                                .get("device_name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                                .to_string();
                            device_id = Some(id.clone());

                            if is_trusted(&id) {
                                trusted = true;
                                touch_device(&id);
                                info!("Trusted device connected: {} ({})", name, id);
                                let resp = json!({"type": "trusted", "device_id": id});
                                let _ = write
                                    .send(Message::Text(resp.to_string().into()))
                                    .await;
                            } else {
                                info!("Unknown device connected: {} ({})", name, id);
                                let resp = json!({
                                    "type": "untrusted",
                                    "message": "Device not paired. Send pair_request to initiate pairing."
                                });
                                let _ = write
                                    .send(Message::Text(resp.to_string().into()))
                                    .await;
                            }
                        }

                        "pair_request" => {
                            if trusted {
                                let resp = json!({"type": "error", "message": "Already paired"});
                                let _ = write
                                    .send(Message::Text(resp.to_string().into()))
                                    .await;
                                continue;
                            }

                            let id = match device_id {
                                Some(ref id) => id.clone(),
                                None => {
                                    let resp = json!({
                                        "type": "error",
                                        "message": "Must identify first"
                                    });
                                    let _ = write
                                        .send(Message::Text(resp.to_string().into()))
                                        .await;
                                    continue;
                                }
                            };

                            let device_name = req
                                .get("device_name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                                .to_string();

                            let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();

                            {
                                let mut state = app_state.lock().unwrap();
                                *state = Some(PendingPair {
                                    device_id: id.clone(),
                                    device_name: device_name.clone(),
                                    responder: resp_tx,
                                });
                            }
                            info!("Pair request from {} ({}), awaiting tray approval", device_name, id);

                            let approved = match tokio::time::timeout(
                                Duration::from_secs(PAIR_TIMEOUT_SECS),
                                resp_rx,
                            )
                            .await
                            {
                                Ok(Ok(true)) => true,
                                _ => false,
                            };

                            {
                                let mut state = app_state.lock().unwrap();
                                *state = None;
                            }

                            if approved {
                                add_device(&id, &device_name);
                                trusted = true;
                                info!("Paired with device: {} ({})", device_name, id);
                                let resp = json!({"type": "pair_accepted", "device_id": id});
                                let _ = write
                                    .send(Message::Text(resp.to_string().into()))
                                    .await;
                            } else {
                                info!("Pairing rejected for: {} ({})", device_name, id);
                                let resp = json!({
                                    "type": "pair_rejected",
                                    "device_id": id,
                                    "reason": "User declined or timeout"
                                });
                                let _ = write
                                    .send(Message::Text(resp.to_string().into()))
                                    .await;
                            }
                        }

                        "ping" => {
                            if !trusted && device_id.is_some() {
                                let resp = json!({
                                    "type": "error",
                                    "message": "Device not trusted. Complete pairing first."
                                });
                                let _ = write
                                    .send(Message::Text(resp.to_string().into()))
                                    .await;
                                continue;
                            }
                            let resp = json!({
                                "type": "pong",
                                "echo": req,
                                "deviceId": peer.to_string()
                            });
                            let resp_text = serde_json::to_string(&resp).unwrap();
                            if let Err(e) =
                                write.send(Message::Text(resp_text.into())).await
                            {
                                warn!("Failed to send pong to {}: {}", peer, e);
                                break;
                            }
                        }

                        "action" => {
                            if !trusted {
                                let rid = req
                                    .get("request_id")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("unknown");
                                let resp = json!({
                                    "type": "error",
                                    "request_id": rid,
                                    "message": "Device not paired. Complete pairing first."
                                });
                                let _ = write
                                    .send(Message::Text(resp.to_string().into()))
                                    .await;
                                continue;
                            }

                            let request_id = req
                                .get("request_id")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                                .to_string();

                            let action_name = req
                                .get("action")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");

                            let empty = json!({});
                            let payload = req.get("payload").unwrap_or(&empty);

                            info!(
                                "Action from {}: action={}, request_id={}",
                                peer, action_name, request_id
                            );

                            let result = ACTIONS.execute(action_name, payload);

                            let resp = match result {
                                Ok(data) => json!({
                                    "type": "action_result",
                                    "request_id": request_id,
                                    "success": true,
                                    "data": data
                                }),
                                Err(e) => json!({
                                    "type": "action_result",
                                    "request_id": request_id,
                                    "success": false,
                                    "error": e.message
                                }),
                            };

                            if let Err(e) = write
                                .send(Message::Text(resp.to_string().into()))
                                .await
                            {
                                warn!("Failed to send action result to {}: {}", peer, e);
                                break;
                            }
                        }

                        _ => {
                            let resp = json!({
                                "type": "error",
                                "message": format!("Unknown message type: {}", msg_type)
                            });
                            let _ = write
                                .send(Message::Text(resp.to_string().into()))
                                .await;
                        }
                    }
                }
                Err(e) => {
                    warn!("Invalid JSON from {}: {}", peer, e);
                    let resp = json!({"type": "error", "message": "Invalid JSON"});
                    let _ = write.send(Message::Text(resp.to_string().into())).await;
                }
            },
            Ok(Message::Close(_)) => {
                if let Some(ref id) = device_id {
                    info!("Connection closed by {} ({})", peer, id);
                } else {
                    info!("Connection closed by {}", peer);
                }
                break;
            }
            Ok(Message::Ping(data)) => {
                let _ = write.send(Message::Pong(data)).await;
            }
            Err(e) => {
                warn!("WebSocket error from {}: {}", peer, e);
                break;
            }
            _ => {}
        }
    }

    if let Some(ref id) = device_id {
        info!("Connection closed: {} ({})", peer, id);
    } else {
        info!("Connection closed: {}", peer);
    }
}

// --- EP-002.5: Windows helpers ---

#[cfg(windows)]
fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(windows)]
fn ensure_single_instance() -> Option<HANDLE> {
    unsafe {
        let wide = to_wide(MUTEX_NAME);
        let handle = CreateMutexW(std::ptr::null(), 1, wide.as_ptr());
        let err = GetLastError();
        if handle.is_null() {
            error!("Failed to create single-instance mutex");
            return None;
        }
        if err == ERROR_ALREADY_EXISTS {
            CloseHandle(handle);
            println!("AutoMatDeck Agent is already running.");
            std::process::exit(0);
        }
        Some(handle)
    }
}

#[cfg(windows)]
fn install_auto_start() {
    let exe = std::env::current_exe().expect("Failed to get executable path");
    let path_str = exe.to_string_lossy().to_string();
    let value = if path_str.contains(' ') {
        format!("\"{}\"", path_str)
    } else {
        path_str
    };

    unsafe {
        let wide_path = to_wide(REGISTRY_PATH);
        let wide_value_name = to_wide(REGISTRY_VALUE_NAME);
        let wide_value = to_wide(&value);

        let mut hkey = std::ptr::null_mut();
        let result = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            wide_path.as_ptr(),
            0,
            KEY_SET_VALUE,
            &mut hkey,
        );
        if result != 0 {
            error!("Failed to open registry key (error {})", result);
            println!("Failed to open registry key. Run as administrator?");
            return;
        }

        let result = RegSetValueExW(
            hkey,
            wide_value_name.as_ptr(),
            0,
            REG_SZ,
            wide_value.as_ptr() as *const u8,
            (wide_value.len() * 2) as u32,
        );
        RegCloseKey(hkey);

        if result == 0 {
            println!("Auto-start registered. Agent will start on next login.");
        } else {
            error!("Failed to set registry value (error {})", result);
            println!("Failed to set registry value (error {})", result);
        }
    }
}

#[cfg(windows)]
fn uninstall_auto_start() {
    unsafe {
        let wide_path = to_wide(REGISTRY_PATH);
        let wide_value_name = to_wide(REGISTRY_VALUE_NAME);

        let mut hkey = std::ptr::null_mut();
        let result = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            wide_path.as_ptr(),
            0,
            KEY_SET_VALUE,
            &mut hkey,
        );
        if result != 0 {
            error!("Failed to open registry key (error {})", result);
            println!("Failed to open registry key.");
            return;
        }

        let result = RegDeleteValueW(hkey, wide_value_name.as_ptr());
        RegCloseKey(hkey);

        if result == 0 {
            println!("Auto-start removed.");
        } else {
            println!("Auto-start was not registered (error {}).", result);
        }
    }
}

#[cfg(windows)]
fn is_auto_start_enabled() -> bool {
    unsafe {
        let wide_path = to_wide(REGISTRY_PATH);
        let wide_value_name = to_wide(REGISTRY_VALUE_NAME);

        let mut hkey = std::ptr::null_mut();
        let result = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            wide_path.as_ptr(),
            0,
            KEY_QUERY_VALUE,
            &mut hkey,
        );
        if result != 0 {
            return false;
        }

        let mut value_type: u32 = 0;
        let mut buffer = [0u16; 512];
        let mut size = (buffer.len() * 2) as u32;
        let result = RegQueryValueExW(
            hkey,
            wide_value_name.as_ptr(),
            std::ptr::null(),
            &mut value_type,
            buffer.as_mut_ptr() as *mut u8,
            &mut size,
        );
        RegCloseKey(hkey);
        result == 0
    }
}

#[cfg(not(windows))]
fn is_auto_start_enabled() -> bool {
    false
}

#[cfg(windows)]
fn handle_cli_args() -> bool {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        match args[1].as_str() {
            "--install" => {
                install_auto_start();
                return true;
            }
            "--uninstall" => {
                uninstall_auto_start();
                return true;
            }
            other => {
                eprintln!("Unknown argument: {}. Usage: {} [--install|--uninstall]", other, args[0]);
                return true;
            }
        }
    }
    false
}

#[cfg(not(windows))]
fn handle_cli_args() -> bool {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        eprintln!("CLI arguments not supported on this platform.");
        return true;
    }
    false
}

fn init_logging() {
    let data_dir = get_data_dir();
    std::fs::create_dir_all(&data_dir).ok();
    let log_path = data_dir.join("agent.log");
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .expect("Failed to open log file");
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .target(env_logger::Target::Pipe(Box::new(log_file)))
        .init();
    info!("AutoMatDeck Agent starting...");
    info!("Log file: {}", log_path.display());

    #[cfg(debug_assertions)]
    {
        println!("AutoMatDeck Agent v0.1.0");
        println!("Logging to: {}", log_path.display());
    }
}

fn make_icon() -> Icon {
    let w = 32u32;
    let h = 32u32;
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let cx = 16i32;
            let cy = 16i32;
            let dx = (x as i32 - cx).abs();
            let dy = (y as i32 - cy).abs();
            let dist = ((dx * dx + dy * dy) as f64).sqrt();
            if dist < 14.0 {
                rgba.push(0x42);
                rgba.push(0x85);
                rgba.push(0xF4);
                rgba.push(0xFF);
            } else if dist < 15.0 {
                rgba.push(0xFF);
                rgba.push(0xFF);
                rgba.push(0xFF);
                rgba.push(0xFF);
            } else {
                rgba.push(0x00);
                rgba.push(0x00);
                rgba.push(0x00);
                rgba.push(0x00);
            }
        }
    }
    Icon::from_rgba(rgba, w, h).expect("Failed to create tray icon")
}

fn open_logs_folder() {
    let path = get_data_dir();
    let _ = std::process::Command::new("explorer")
        .arg(path.to_string_lossy().to_string())
        .spawn();
}

fn create_tray_icon() -> MenuItems {
    let status = MenuItem::with_id("status", "Status: Running", false, None);
    let start_on_login = CheckMenuItem::with_id("start_on_login", "Start with Windows", true, false, None);
    let logs = MenuItem::with_id("logs", "Open Logs", true, None);
    let exit = MenuItem::with_id("exit", "Exit", true, None);

    let icon = make_icon();

    let menu = build_default_menu(&status, &start_on_login, &logs, &exit);

    let tray = TrayIconBuilder::new()
        .with_tooltip("AutoMatDeck Agent")
        .with_icon(icon)
        .with_menu(Box::new(menu))
        .build()
        .expect("Failed to create tray icon");

    // Sync checkbox with current registry state
    #[cfg(windows)]
    {
        let enabled = is_auto_start_enabled();
        start_on_login.set_checked(enabled);
    }

    MenuItems { tray, status, start_on_login, logs, exit }
}

fn build_default_menu(
    status: &MenuItem,
    start_on_login: &CheckMenuItem,
    logs: &MenuItem,
    exit: &MenuItem,
) -> Menu {
    let sep = PredefinedMenuItem::separator();
    Menu::with_items(&[status, &sep, start_on_login, logs, exit])
        .expect("Failed to build tray menu")
}

fn build_pending_menu(
    status: &MenuItem,
    start_on_login: &CheckMenuItem,
    logs: &MenuItem,
    exit: &MenuItem,
    device_name: &str,
) -> Menu {
    let sep = PredefinedMenuItem::separator();
    let info = MenuItem::with_id("pending_info", format!("⚠ Pending: {}", device_name), false, None);
    let approve = MenuItem::with_id("approve", "  Approve", true, None);
    let reject = MenuItem::with_id("reject", "  Reject", true, None);
    Menu::with_items(&[status, &sep, &info, &approve, &reject, &sep, start_on_login, logs, exit])
        .expect("Failed to build pending tray menu")
}

fn run_message_pump(
    shutdown_tx: tokio::sync::watch::Sender<bool>,
    app_state: AppState,
    menu: MenuItems,
) {
    info!("System tray icon active.");

    let mut showing_pending = false;

    loop {
        unsafe {
            let mut msg: MSG = std::mem::zeroed();
            while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id == *menu.exit.id() {
                info!("Exit requested from tray menu");
                let _ = shutdown_tx.send(true);
                return;
            } else if event.id == *menu.logs.id() {
                info!("Open Logs requested from tray menu");
                open_logs_folder();
            } else if event.id == *menu.start_on_login.id() {
                let checked = menu.start_on_login.is_checked();
                if checked {
                    uninstall_auto_start();
                    menu.start_on_login.set_checked(false);
                    info!("Auto-start disabled");
                } else {
                    install_auto_start();
                    menu.start_on_login.set_checked(true);
                    info!("Auto-start enabled");
                }
            } else if event.id == "approve" {
                let pair = app_state.lock().unwrap().take();
                if let Some(p) = pair {
                    info!("Pairing approved via tray: {} ({})", p.device_name, p.device_id);
                    let _ = p.responder.send(true);
                }
            } else if event.id == "reject" {
                let pair = app_state.lock().unwrap().take();
                if let Some(p) = pair {
                    info!("Pairing rejected via tray: {} ({})", p.device_name, p.device_id);
                    let _ = p.responder.send(false);
                }
            }
        }

        // Poll AppState to update tray menu when pair request arrives or resolves
        let has_pending = app_state.lock().unwrap().is_some();
        if has_pending != showing_pending {
            showing_pending = has_pending;
            if has_pending {
                let device_name = app_state.lock().unwrap().as_ref().unwrap().device_name.clone();
                let new_menu = build_pending_menu(
                    &menu.status, &menu.start_on_login, &menu.logs, &menu.exit, &device_name,
                );
                menu.tray.set_menu(Some(Box::new(new_menu)));
            } else {
                let new_menu = build_default_menu(
                    &menu.status, &menu.start_on_login, &menu.logs, &menu.exit,
                );
                menu.tray.set_menu(Some(Box::new(new_menu)));
            }
        }

        std::thread::sleep(Duration::from_millis(50));
    }
}

// --- Async server ---

async fn async_main(
    mut shutdown_rx: tokio::sync::watch::Receiver<bool>,
    app_state: AppState,
) {
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    let device_id = format!("amd-{}", &hostname);

    let mut advertisers: Vec<Box<dyn AdvertisementProvider>> = Vec::new();
    advertisers.push(Box::new(discovery::MdnsAnnouncer::new(
        device_id.clone(),
        hostname.clone(),
        PORT,
    )));

    for advertiser in advertisers.iter_mut() {
        match advertiser.start() {
            Ok(()) => info!(
                "[{}] Started: device_id={}",
                advertiser.provider_name(),
                advertiser.device_id()
            ),
            Err(e) => warn!(
                "[{}] Failed to start: {}",
                advertiser.provider_name(),
                e
            ),
        }
    }

    info!(
        "Desktop agent started. Hostname: {}, Device ID: {}, Listening on port {}",
        hostname, device_id, PORT
    );
    info!("Trusted devices stored at: {}", store_path().display());

    let addr: SocketAddr = ([0, 0, 0, 0], PORT).into();
    let listener = TcpListener::bind(&addr)
        .await
        .expect("Failed to bind WebSocket server");

    info!("WebSocket server listening on ws://{}", addr);

    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, peer)) => {
                        info!("New connection from {}", peer);
                        tokio::spawn(handle_connection(stream, peer, app_state.clone()));
                    }
                    Err(e) => {
                        warn!("Accept error: {}", e);
                        break;
                    }
                }
            }
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    info!("Shutdown signal received, stopping server...");
                    break;
                }
            }
        }
    }
}

// --- Main ---

fn main() {
    #[cfg(windows)]
    {
        let _mutex = ensure_single_instance();
        if handle_cli_args() {
            return;
        }
    }

    init_logging();

    let app_state: AppState = Arc::new(Mutex::new(None));
    let menu = create_tray_icon();

    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    let app_state_for_server = app_state.clone();
    let server_handle = std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        rt.block_on(async_main(shutdown_rx, app_state_for_server));
    });

    run_message_pump(shutdown_tx, app_state, menu);

    info!("Shutting down...");
    let _ = server_handle.join();
    info!("Goodbye.");
}
