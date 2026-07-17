#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod actions;
mod agent;
mod command;
mod discovery;
mod editor;
mod execution;
mod gui;
mod model;
mod observer;
mod pairing;
mod projection;
mod projection_transport;
mod repository;
mod state;
mod tray;
mod trigger_execution;
mod trigger_validation;
mod workflow_validation;

use std::sync::Arc;

use agent::PairState;
use gui::DesktopApp;
use log::info;
use state::SharedRuntime;

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
    let shared_runtime: SharedRuntime = state::new_shared(&*repo);

    let (shutdown_tx, shutdown_rx_from_main) = tokio::sync::watch::channel(false);

    let (projection_transport, projection_state_rx) =
        projection_transport::ProjectionTransportPublisher::new();

    let (css_transport, css_state_rx) =
        projection_transport::ControlSurfaceTransportPublisher::new();

    let ps_for_server = pair_state.clone();
    let pm_for_server = pairing_manager.clone();
    let rt_for_server = shared_runtime.clone();
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
                rt_for_server,
                repo_for_server,
                projection_state_rx,
                css_state_rx,
            ));
        })
        .expect("Failed to spawn server thread");

    #[cfg(windows)]
    let obs_rt = shared_runtime.clone();
    #[cfg(windows)]
    let observation_cell = Arc::new(projection::TransitionCell::new());
    #[cfg(windows)]
    let shutdown_rx_for_observer = shutdown_rx_from_main.clone();
    #[cfg(windows)]
    let cell_for_observer = observation_cell.clone();
    #[cfg(windows)]
    let observer_handle = std::thread::Builder::new()
        .name("context-observer".into())
        .spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(200));
            loop {
                let shutdown = match shutdown_rx_for_observer.has_changed() {
                    Ok(true) => *shutdown_rx_for_observer.borrow(),
                    Ok(false) => false,
                    Err(_) => true,
                };
                if shutdown {
                    break;
                }
                let observation = observer::ForegroundObserver::current_context();
                if let Some(snapshot) = observer::successful_observation(observation) {
                    let transition = {
                        let mut guard = obs_rt.lock().unwrap();
                        guard.apply_context_observation(snapshot)
                    };
                    cell_for_observer.store(transition);
                }
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
        })
        .expect("Failed to spawn observer thread");

    let cell_for_projection = observation_cell.clone();
    let shutdown_rx_for_projection = shutdown_rx_from_main.clone();
    let rt_for_projection = shared_runtime.clone();
    let projection_handle = std::thread::Builder::new()
        .name("projection".into())
        .spawn(move || {
            let mut aps_policy = projection::PublicationPolicy::new();
            let mut css_policy = projection::ControlSurfacePublicationPolicy::new();
            let aps_publisher: Arc<dyn projection::ProjectionPublisher> =
                Arc::new(projection_transport);
            loop {
                let shutdown = match shutdown_rx_for_projection.has_changed() {
                    Ok(true) => *shutdown_rx_for_projection.borrow(),
                    Ok(false) => false,
                    Err(_) => true,
                };
                if shutdown {
                    break;
                }
                // Wake on notification; fall back to 200ms timeout for shutdown check
                let transition = cell_for_projection
                    .wait_and_take_timeout(std::time::Duration::from_millis(200));
                if let Some(transition) = transition {
                    // APS derivation
                    let aps_proj = projection::project(&transition);
                    if aps_policy.should_publish(&aps_proj) {
                        aps_publisher.publish(&aps_proj);
                    }

                    // CSS derivation — lock only to read active_profile_id & profiles
                    let css = {
                        let runtime = rt_for_projection.lock().unwrap();
                        projection::derive_control_surface(
                            runtime.runtime.active_profile_id.as_ref(),
                            &runtime.app.document.profiles,
                        )
                    };

                    match &css {
                        projection::DerivationResult::Failed => {
                            log::warn!("Control surface derivation failed for active profile");
                        }
                        projection::DerivationResult::Published(_) => {
                            if css_policy.should_publish(&css) {
                                css_transport.publish_derivation(&css);
                            }
                        }
                    }
                }
            }
        })
        .expect("Failed to spawn projection thread");

    let ps_for_tray = pair_state.clone();
    let rt_for_tray = shared_runtime.clone();
    let tray_handle = std::thread::Builder::new()
        .name("tray-pump".into())
        .spawn(move || {
            let menu = tray::create_menu_items();
            {
                let mut rt = rt_for_tray.lock().unwrap();
                rt.app.server_running = true;
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

    let app = DesktopApp::new(shared_runtime.clone(), pairing_manager.clone(), repo);

    info!("Desktop Studio window opened.");

    let result = eframe::run_native(
        "AutoMatDeck Desktop Studio",
        native_options,
        Box::new(|_cc| Ok(Box::new(app))),
    );

    info!("GUI closed, initiating shutdown...");
    drop(shutdown_rx_from_main);
    let _ = server_handle.join();
    let _ = projection_handle.join();
    #[cfg(windows)]
    let _ = observer_handle.join();
    let _ = tray_handle.join();
    info!("Goodbye.");

    result
}
