use std::time::Duration;

use log::{info, error};
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

use crate::device_store;
use crate::agent::PairState;

#[cfg(windows)]
const MUTEX_NAME: &str = "Local\\AutoMatDeck_Agent";
#[cfg(windows)]
const REGISTRY_PATH: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
#[cfg(windows)]
const REGISTRY_VALUE_NAME: &str = "AutoMatDeck Agent";

pub struct MenuItems {
    pub tray: TrayIcon,
    pub status: MenuItem,
    pub show_window: MenuItem,
    pub start_on_login: CheckMenuItem,
    pub logs: MenuItem,
    pub exit: MenuItem,
}

#[cfg(windows)]
pub fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(windows)]
pub fn ensure_single_instance() -> Option<HANDLE> {
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
pub fn install_auto_start() {
    let exe = std::env::current_exe().expect("Failed to get executable path");
    let path_str = exe.to_string_lossy().to_string();
    let value = if path_str.contains(' ') { format!("\"{}\"", path_str) } else { path_str };

    unsafe {
        let wide_path = to_wide(REGISTRY_PATH);
        let wide_value_name = to_wide(REGISTRY_VALUE_NAME);
        let wide_value = to_wide(&value);

        let mut hkey = std::ptr::null_mut();
        let result = RegOpenKeyExW(HKEY_CURRENT_USER, wide_path.as_ptr(), 0, KEY_SET_VALUE, &mut hkey);
        if result != 0 {
            error!("Failed to open registry key (error {})", result);
            return;
        }

        let result = RegSetValueExW(
            hkey, wide_value_name.as_ptr(), 0, REG_SZ,
            wide_value.as_ptr() as *const u8, (wide_value.len() * 2) as u32,
        );
        RegCloseKey(hkey);

        if result == 0 {
            info!("Auto-start registered.");
        } else {
            error!("Failed to set registry value (error {})", result);
        }
    }
}

#[cfg(windows)]
pub fn uninstall_auto_start() {
    unsafe {
        let wide_path = to_wide(REGISTRY_PATH);
        let wide_value_name = to_wide(REGISTRY_VALUE_NAME);

        let mut hkey = std::ptr::null_mut();
        let result = RegOpenKeyExW(HKEY_CURRENT_USER, wide_path.as_ptr(), 0, KEY_SET_VALUE, &mut hkey);
        if result != 0 { return; }

        let result = RegDeleteValueW(hkey, wide_value_name.as_ptr());
        RegCloseKey(hkey);
        if result == 0 {
            info!("Auto-start removed.");
        }
    }
}

#[cfg(windows)]
pub fn is_auto_start_enabled() -> bool {
    unsafe {
        let wide_path = to_wide(REGISTRY_PATH);
        let wide_value_name = to_wide(REGISTRY_VALUE_NAME);

        let mut hkey = std::ptr::null_mut();
        let result = RegOpenKeyExW(HKEY_CURRENT_USER, wide_path.as_ptr(), 0, KEY_QUERY_VALUE, &mut hkey);
        if result != 0 { return false; }

        let mut value_type: u32 = 0;
        let mut buffer = [0u16; 512];
        let mut size = (buffer.len() * 2) as u32;
        let result = RegQueryValueExW(
            hkey, wide_value_name.as_ptr(), std::ptr::null(),
            &mut value_type, buffer.as_mut_ptr() as *mut u8, &mut size,
        );
        RegCloseKey(hkey);
        result == 0
    }
}

#[cfg(not(windows))]
pub fn is_auto_start_enabled() -> bool { false }

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
                rgba.extend_from_slice(&[0x42, 0x85, 0xF4, 0xFF]);
            } else if dist < 15.0 {
                rgba.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]);
            } else {
                rgba.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
            }
        }
    }
    Icon::from_rgba(rgba, w, h).expect("Failed to create tray icon")
}

pub fn create_menu_items() -> MenuItems {
    let status = MenuItem::with_id("status", "Status: Running", false, None);
    let show_window = MenuItem::with_id("show_window", "Open Desktop Studio", true, None);
    let start_on_login = CheckMenuItem::with_id("start_on_login", "Start with Windows", true, false, None);
    let logs = MenuItem::with_id("logs", "Open Logs", true, None);
    let exit = MenuItem::with_id("exit", "Exit", true, None);

    let icon = make_icon();

    let sep = PredefinedMenuItem::separator();
    let menu = Menu::with_items(&[&status, &sep, &show_window, &start_on_login, &logs, &exit])
        .expect("Failed to build tray menu");

    let tray = TrayIconBuilder::new()
        .with_tooltip("AutoMatDeck Agent")
        .with_icon(icon)
        .with_menu(Box::new(menu))
        .build()
        .expect("Failed to create tray icon");

    #[cfg(windows)]
    start_on_login.set_checked(is_auto_start_enabled());

    MenuItems { tray, status, show_window, start_on_login, logs, exit }
}

pub fn open_logs_folder() {
    let path = device_store::get_data_dir();
    let _ = std::process::Command::new("explorer")
        .arg(path.to_string_lossy().to_string())
        .spawn();
}

pub fn run_message_pump(
    shutdown_tx: tokio::sync::watch::Sender<bool>,
    pair_state: PairState,
    menu: MenuItems,
) {
    info!("System tray message pump started.");

    loop {
        #[cfg(windows)]
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
            } else if event.id == *menu.show_window.id() {
                info!("Show window requested from tray menu");
            } else if event.id == *menu.logs.id() {
                open_logs_folder();
            } else if event.id == *menu.start_on_login.id() {
                let checked = menu.start_on_login.is_checked();
                #[cfg(windows)]
                {
                    if checked {
                        uninstall_auto_start();
                        menu.start_on_login.set_checked(false);
                    } else {
                        install_auto_start();
                        menu.start_on_login.set_checked(true);
                    }
                }
                #[cfg(not(windows))]
                menu.start_on_login.set_checked(false);
            } else if event.id == "approve" {
                let pair = pair_state.lock().unwrap().take();
                if let Some(p) = pair {
                    info!("[PAIR] User approved via tray: {} ({})", p.device_name, p.device_id);
                    let _ = p.responder.send(true);
                } else {
                    info!("[PAIR] Approve clicked but no pending pair request");
                }
            } else if event.id == "reject" {
                let pair = pair_state.lock().unwrap().take();
                if let Some(p) = pair {
                    info!("[PAIR] User rejected via tray: {} ({})", p.device_name, p.device_id);
                    let _ = p.responder.send(false);
                } else {
                    info!("[PAIR] Reject clicked but no pending pair request");
                }
            }
        }

        let has_pending = if let Ok(state) = pair_state.lock() {
            state.is_some()
        } else {
            false
        };

        if has_pending {
            let status_text = if let Ok(state) = pair_state.lock() {
                state.as_ref().map(|p| format!("⚠ Pending: {}", p.device_name))
                    .unwrap_or_else(|| "Status: Running".into())
            } else {
                "Status: Running".into()
            };
            menu.status.set_text(status_text);

            let sep = PredefinedMenuItem::separator();
            let info = MenuItem::with_id("pending_info", "Pair request pending...", false, None);
            let approve = MenuItem::with_id("approve", "  Approve", true, None);
            let reject = MenuItem::with_id("reject", "  Reject", true, None);
            let new_menu = Menu::with_items(&[
                &menu.status, &sep, &info, &approve, &reject, &sep,
                &menu.show_window, &menu.start_on_login, &menu.logs, &menu.exit,
            ]).expect("Failed to build pending tray menu");
            menu.tray.set_menu(Some(Box::new(new_menu)));
        } else {
            menu.status.set_text("Status: Running");
            let sep = PredefinedMenuItem::separator();
            let new_menu = Menu::with_items(&[
                &menu.status, &sep,
                &menu.show_window, &menu.start_on_login, &menu.logs, &menu.exit,
            ]).expect("Failed to build default tray menu");
            menu.tray.set_menu(Some(Box::new(new_menu)));
        }

        std::thread::sleep(Duration::from_millis(100));
    }
}
