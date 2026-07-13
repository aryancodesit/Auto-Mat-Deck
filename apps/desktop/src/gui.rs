use std::sync::Arc;

use egui::{Color32, Context, Frame, RichText, ScrollArea};
use log::info;

use crate::command::{Command, CommandError};
use crate::editor::EditorUi;
use crate::pairing::SharedPairingManager;
use crate::repository::DocumentStore;
use crate::state::{AppState, SharedState, Tab};

pub struct DesktopApp {
    pub state: SharedState,
    pairing_manager: SharedPairingManager,
    store: Arc<dyn DocumentStore>,
    theme: Theme,
    rename_device_id: String,
    rename_buffer: String,
    pub editor: EditorUi,
}

#[derive(PartialEq)]
enum Theme {
    Light,
    Dark,
}

impl Default for Theme {
    fn default() -> Self {
        Theme::Dark
    }
}

impl DesktopApp {
    pub fn new(
        state: SharedState,
        pairing_manager: SharedPairingManager,
        store: Arc<dyn DocumentStore>,
    ) -> Self {
        Self {
            state,
            pairing_manager,
            store,
            theme: Theme::Dark,
            rename_device_id: String::new(),
            rename_buffer: String::new(),
            editor: EditorUi::new(),
        }
    }

    fn persist(&self, state: &AppState) {
        state.persist(&*self.store);
    }

    /// Single orchestration path for editor mutations.
    /// Locks state, dispatches, persists on success, returns error.
    fn dispatch_editor(&self, cmd: &Command) -> Result<(), CommandError> {
        let mut state = self.state.lock().unwrap();
        state.dispatch(cmd)?;
        self.persist(&state);
        Ok(())
    }
}

impl eframe::App for DesktopApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        match self.theme {
            Theme::Dark => ctx.set_visuals(egui::Visuals::dark()),
            Theme::Light => ctx.set_visuals(egui::Visuals::light()),
        }

        egui::TopBottomPanel::top("title_bar")
            .frame(Frame {
                fill: Color32::from_rgb(30, 30, 35),
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("AutoMatDeck Desktop Studio");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let running = self.state.lock().unwrap().server_running;
                        let label = if running {
                            "● Running"
                        } else {
                            "○ Stopped"
                        };
                        let color = if running {
                            Color32::GREEN
                        } else {
                            Color32::RED
                        };
                        ui.colored_label(color, label);
                    });
                });
            });

        egui::TopBottomPanel::bottom("status_bar")
            .frame(Frame {
                fill: Color32::from_rgb(30, 30, 35),
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(format!("v{}", env!("CARGO_PKG_VERSION")));
                    ui.separator();
                    let device_count = self.state.lock().unwrap().device_count();
                    ui.label(format!("{} trusted devices", device_count));
                });
            });

        egui::SidePanel::left("tabs")
            .resizable(false)
            .default_width(160.0)
            .frame(Frame {
                fill: Color32::from_rgb(25, 25, 30),
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(12.0);
                });
                let mut state = self.state.lock().unwrap();
                ui.selectable_value(&mut state.selected_tab, Tab::Dashboard, "📊  Dashboard");
                ui.selectable_value(&mut state.selected_tab, Tab::Editor, "🖊  Editor");
                ui.selectable_value(&mut state.selected_tab, Tab::Devices, "📱  Devices");
                ui.selectable_value(&mut state.selected_tab, Tab::Pairing, "🔗  Pairing");
                ui.selectable_value(&mut state.selected_tab, Tab::Settings, "⚙  Settings");
                ui.selectable_value(&mut state.selected_tab, Tab::About, "ℹ  About");
                drop(state);
            });

        let active_tab;
        let server_running;
        {
            let gs = self.state.lock().unwrap();
            active_tab = gs.selected_tab.clone();
            server_running = gs.server_running;
        }
        egui::CentralPanel::default().show(ctx, |ui| match active_tab {
            Tab::Dashboard => self.show_dashboard(ui, server_running),
            Tab::Editor => self.show_editor(ui),
            Tab::Devices => self.show_devices(ui),
            Tab::Pairing => self.show_pairing(ui),
            Tab::Settings => self.show_settings(ui),
            Tab::About => self.show_about(ui),
        });
    }
}

impl DesktopApp {
    fn show_editor(&mut self, ui: &mut egui::Ui) {
        let doc = {
            let state = self.state.lock().unwrap();
            state.document.clone()
        };

        let cmd = self.editor.show(ui, &doc);

        if let Some(cmd) = cmd {
            let result = self.dispatch_editor(&cmd);
            match result {
                Ok(()) => {
                    self.editor.last_command_error = None;
                }
                Err(e) => {
                    self.editor.last_command_error = Some(e);
                }
            }
        }
    }

    fn show_dashboard(&self, ui: &mut egui::Ui, server_running: bool) {
        ui.heading("Dashboard");
        ui.separator();

        Frame::group(ui.style()).show(ui, |ui| {
            ui.label(RichText::new("Status").strong());
            let status = if server_running { "Running" } else { "Stopped" };
            ui.label(format!("Server: {}", status));
            ui.label(format!("Port: {}", crate::agent::PORT));
            ui.label(format!("Data dir: {}", self.store.data_dir().display()));
        });

        ui.add_space(8.0);

        Frame::group(ui.style()).show(ui, |ui| {
            ui.label(RichText::new("Recent Actions").strong());
            ui.separator();
            ui.label("No actions yet.");
        });
    }

    fn show_devices(&self, ui: &mut egui::Ui) {
        ui.heading("Trusted Devices");
        ui.separator();

        let devices = {
            let state = self.state.lock().unwrap();
            state.devices().to_vec()
        };

        if devices.is_empty() {
            ui.label("No devices paired yet.");
            ui.label("Pair a device from the mobile app.");
            return;
        }

        ScrollArea::vertical().show(ui, |ui| {
            for device in &devices {
                Frame::group(ui.style()).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.strong(&device.device_name);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Forget").clicked() {
                                let mut state = self.state.lock().unwrap();
                                state.forget_device(device.device_id.as_str());
                                self.persist(&state);
                                info!(
                                    "Forgot device: {} ({})",
                                    device.device_name, device.device_id
                                );
                            }
                        });
                    });
                    ui.label(format!("ID: {}", device.device_id));
                    ui.label(format!("Last seen: {}", device.last_seen));
                    ui.label(format!("Paired: {}", device.paired_at));
                });
                ui.add_space(4.0);
            }
        });
    }

    fn show_pairing(&mut self, ui: &mut egui::Ui) {
        ui.heading("Pair a Device");
        ui.separator();

        let snap = self.pairing_manager.snapshot();

        if snap.is_none() {
            // No active session — show generate button
            if ui.button("Generate Pairing Code").clicked() {
                let device_id = "amd-desktop-studio";
                let hostname = hostname::get()
                    .map(|h| h.to_string_lossy().to_string())
                    .unwrap_or_else(|_| "unknown".to_string());
                let port = crate::agent::PORT;
                self.pairing_manager
                    .generate_session(device_id, &hostname, port);
            }
            return;
        }

        let session = snap.unwrap();

        if session.consumed {
            ui.label("This pairing code has already been used.");
            if ui.button("Generate New Code").clicked() {
                let device_id = "amd-desktop-studio";
                let hostname = hostname::get()
                    .map(|h| h.to_string_lossy().to_string())
                    .unwrap_or_else(|_| "unknown".to_string());
                let port = crate::agent::PORT;
                self.pairing_manager
                    .generate_session(device_id, &hostname, port);
            }
            return;
        }

        if session.cancelled {
            ui.label("Pairing was cancelled.");
            if ui.button("Generate New Code").clicked() {
                let device_id = "amd-desktop-studio";
                let hostname = hostname::get()
                    .map(|h| h.to_string_lossy().to_string())
                    .unwrap_or_else(|_| "unknown".to_string());
                let port = crate::agent::PORT;
                self.pairing_manager
                    .generate_session(device_id, &hostname, port);
            }
            return;
        }

        // Check expiry
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let expired = now > session.expires_at;

        if expired {
            ui.colored_label(
                egui::Color32::RED,
                "Pairing code expired. Generate a new one.",
            );
            if ui.button("Generate New Code").clicked() {
                let device_id = "amd-desktop-studio";
                let hostname = hostname::get()
                    .map(|h| h.to_string_lossy().to_string())
                    .unwrap_or_else(|_| "unknown".to_string());
                let port = crate::agent::PORT;
                self.pairing_manager
                    .generate_session(device_id, &hostname, port);
            }
            return;
        }

        // Active session
        let remaining = session.expires_at - now;
        let mins = remaining / 60;
        let secs = remaining % 60;

        // QR code
        if session.qr_size > 0 {
            Frame::group(ui.style()).show(ui, |ui| {
                ui.label(RichText::new("Scan with Mobile App").strong());
                ui.add_space(4.0);

                let cell_size = 6.0;
                let total_size = session.qr_size as f32 * cell_size;

                let (_, response) = ui
                    .allocate_exact_size(egui::vec2(total_size, total_size), egui::Sense::hover());

                let painter = ui.painter_at(response.rect);
                let origin = response.rect.min;

                for (y, row) in session.qr_matrix.iter().enumerate() {
                    for (x, &dark) in row.iter().enumerate() {
                        if dark {
                            let x = origin.x + x as f32 * cell_size;
                            let y = origin.y + y as f32 * cell_size;
                            painter.rect_filled(
                                egui::Rect::from_min_size(
                                    egui::pos2(x, y),
                                    egui::vec2(cell_size, cell_size),
                                ),
                                0.0,
                                egui::Color32::WHITE,
                            );
                        }
                    }
                }
            });
            ui.add_space(8.0);
        } else {
            ui.colored_label(egui::Color32::GRAY, "QR code unavailable");
            ui.add_space(8.0);
        }

        // OTP
        Frame::group(ui.style()).show(ui, |ui| {
            ui.label(RichText::new("Pairing Code").strong());
            ui.add_space(4.0);
            ui.label("Enter this code on your mobile device:");
            ui.add_space(2.0);
            ui.heading(
                RichText::new(&session.otp)
                    .size(48.0)
                    .color(egui::Color32::from_rgb(0, 200, 100)),
            );
            ui.add_space(4.0);
            ui.label(format!("Expires in: {}m {}s", mins, secs));
        });

        ui.add_space(8.0);

        // Cancel button
        if ui.button("Cancel Pairing").clicked() {
            self.pairing_manager.cancel_session();
        }
    }

    fn show_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("Settings");
        ui.separator();

        Frame::group(ui.style()).show(ui, |ui| {
            ui.label(RichText::new("General").strong());

            let mut auto_start = crate::tray::is_auto_start_enabled();
            if ui.checkbox(&mut auto_start, "Start with Windows").changed() {
                if auto_start {
                    crate::tray::install_auto_start();
                } else {
                    crate::tray::uninstall_auto_start();
                }
            }
        });

        ui.add_space(8.0);

        Frame::group(ui.style()).show(ui, |ui| {
            ui.label(RichText::new("Appearance").strong());
            let mut is_dark = self.theme == Theme::Dark;
            if ui.checkbox(&mut is_dark, "Dark mode").changed() {
                self.theme = if is_dark { Theme::Dark } else { Theme::Light };
            }
        });

        ui.add_space(8.0);

        Frame::group(ui.style()).show(ui, |ui| {
            ui.label(RichText::new("Server").strong());
            ui.label(format!("Port: {}", crate::agent::PORT));
            ui.label("Restart required to change port.");
        });

        ui.add_space(8.0);

        ui.label("(Window always visible in v0.2)");
        if ui.button("Close").clicked() {
            std::process::exit(0);
        }
    }

    fn show_about(&self, ui: &mut egui::Ui) {
        ui.heading("About");
        ui.separator();

        Frame::group(ui.style()).show(ui, |ui| {
            ui.label(RichText::new("AutoMatDeck Desktop Studio").strong());
            ui.label(format!("Version: {}", env!("CARGO_PKG_VERSION")));
            ui.label("Simple. Personal. Local-first.");
            ui.label("");
            ui.label("Built with Rust + egui");
            #[cfg(windows)]
            ui.label("Platform: Windows");
        });
    }
}
