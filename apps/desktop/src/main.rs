#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod actions;
mod agent;
mod command;
mod discovery;
mod editor;
mod gui;
mod model;
mod pairing;
mod repository;
mod state;
mod tray;

use std::sync::Arc;

use agent::PairState;
use gui::DesktopApp;
use log::info;

use crate::pairing::SharedPairingManager;
use crate::repository::DocumentStore;
use crate::repository::JsonRepository;

fn init_logging(store: &dyn DocumentStore) {
    let log_path = store.data_dir().join("agent.log");
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .expect("Failed to open log file");
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .target(env_logger::Target::Pipe(Box::new(log_file)))
        .init();
    info!(
        "AutoMatDeck Agent v{} starting...",
        env!("CARGO_PKG_VERSION")
    );
    info!("Log file: {}", log_path.display());
}

fn main() -> eframe::Result<()> {
    #[cfg(windows)]
    {
        let _mutex = tray::ensure_single_instance();
    }

    let repo: Arc<dyn DocumentStore> = Arc::new(JsonRepository::new());
    init_logging(&*repo);

    let pair_state: PairState = Arc::new(std::sync::Mutex::new(None));
    let pairing_manager: SharedPairingManager = Arc::new(pairing::PairingManager::new());
    let app_state = state::new_shared(&*repo);

    let (shutdown_tx, shutdown_rx_from_main) = tokio::sync::watch::channel(false);

    let ps_for_server = pair_state.clone();
    let pm_for_server = pairing_manager.clone();
    let as_for_server = app_state.clone();
    let repo_for_server = repo.clone();
    let shutdown_rx_for_server = shutdown_rx_from_main.clone();
    let server_handle = std::thread::Builder::new()
        .name("agent-server".into())
        .spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
            rt.block_on(agent::run_server(
                shutdown_rx_for_server,
                ps_for_server,
                pm_for_server,
                as_for_server,
                repo_for_server,
            ));
        })
        .expect("Failed to spawn server thread");

    let ps_for_tray = pair_state.clone();
    let as_for_tray = app_state.clone();
    let tray_handle = std::thread::Builder::new()
        .name("tray-pump".into())
        .spawn(move || {
            let menu = tray::create_menu_items();
            {
                let mut state = as_for_tray.lock().unwrap();
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

    let app = DesktopApp::new(app_state.clone(), pairing_manager.clone(), repo);

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
