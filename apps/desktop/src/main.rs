#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod actions;
mod agent;
mod device_store;
mod discovery;
mod gui;
mod tray;

use std::sync::{Arc, Mutex};

use gui::{DesktopApp, GuiState};
use agent::PairState;
use log::info;

fn init_logging() {
    let data_dir = device_store::get_data_dir();
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
    info!("AutoMatDeck Agent v{} starting...", env!("CARGO_PKG_VERSION"));
    info!("Log file: {}", log_path.display());
}

fn main() -> eframe::Result<()> {
    #[cfg(windows)]
    {
        let _mutex = tray::ensure_single_instance();
    }

    init_logging();

    let gui_state = Arc::new(Mutex::new(GuiState::default()));
    let pair_state: PairState = Arc::new(Mutex::new(None));

    let (shutdown_tx, shutdown_rx_from_main) = tokio::sync::watch::channel(false);

    let ps_for_server = pair_state.clone();
    let shutdown_rx_for_server = shutdown_rx_from_main.clone();
    let server_handle = std::thread::Builder::new()
        .name("agent-server".into())
        .spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
            rt.block_on(agent::run_server(shutdown_rx_for_server, ps_for_server));
        })
        .expect("Failed to spawn server thread");

    let ps_for_tray = pair_state.clone();
    let gs_for_tray = gui_state.clone();
    let tray_handle = std::thread::Builder::new()
        .name("tray-pump".into())
        .spawn(move || {
            let menu = tray::create_menu_items();
            {
                let mut state = gs_for_tray.lock().unwrap();
                state.server_running = true;
            }
            tray::run_message_pump(shutdown_tx, ps_for_tray, menu);
        })
        .expect("Failed to spawn tray thread");

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("AutoMatDeck Desktop Studio")
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([600.0, 400.0]),
        ..Default::default()
    };

    let app = DesktopApp::new(gui_state.clone());

    info!("Desktop Studio window opened.");

    let result = eframe::run_native(
        "AutoMatDeck Desktop Studio",
        native_options,
        Box::new(|_cc| Ok(Box::new(app))),
    );

    info!("GUI closed, initiating shutdown...");
    drop(shutdown_rx_from_main);
    let _ = server_handle.join();
    let _ = tray_handle.join();
    info!("Goodbye.");

    result
}
