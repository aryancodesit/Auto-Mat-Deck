use std::sync::{Arc, Mutex};

use egui::{Color32, Context, Frame, RichText, ScrollArea};
use log::info;

use crate::device_store;

#[derive(Default)]
pub struct GuiState {
    pub server_running: bool,
    pub selected_tab: Tab,
}

#[derive(Clone, PartialEq)]
pub enum Tab {
    Dashboard,
    Devices,
    Settings,
    About,
}

impl Default for Tab {
    fn default() -> Self { Tab::Dashboard }
}

pub struct DesktopApp {
    pub state: Arc<Mutex<GuiState>>,
    theme: Theme,
    rename_device_id: String,
    rename_buffer: String,
}

#[derive(PartialEq)]
enum Theme { Light, Dark }

impl Default for Theme {
    fn default() -> Self { Theme::Dark }
}

impl DesktopApp {
    pub fn new(state: Arc<Mutex<GuiState>>) -> Self {
        Self { state, theme: Theme::Dark, rename_device_id: String::new(), rename_buffer: String::new() }
    }
}

impl eframe::App for DesktopApp {
    fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {

        match self.theme {
            Theme::Dark => ctx.set_visuals(egui::Visuals::dark()),
            Theme::Light => ctx.set_visuals(egui::Visuals::light()),
        }

        egui::TopBottomPanel::top("title_bar").frame(Frame {
            fill: Color32::from_rgb(30, 30, 35),
            ..Default::default()
        }).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("AutoMatDeck Desktop Studio");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let running = self.state.lock().unwrap().server_running;
                    let label = if running { "● Running" } else { "○ Stopped" };
                    let color = if running { Color32::GREEN } else { Color32::RED };
                    ui.colored_label(color, label);
                });
            });
        });

        egui::TopBottomPanel::bottom("status_bar").frame(Frame {
            fill: Color32::from_rgb(30, 30, 35),
            ..Default::default()
        }).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("v{}", env!("CARGO_PKG_VERSION")));
                ui.separator();
                let device_count = device_store::load_devices().len();
                ui.label(format!("{} trusted devices", device_count));
            });
        });

        egui::SidePanel::left("tabs").resizable(false).default_width(160.0).frame(Frame {
            fill: Color32::from_rgb(25, 25, 30),
            ..Default::default()
        }).show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(12.0);
            });
            let mut state = self.state.lock().unwrap();
            ui.selectable_value(&mut state.selected_tab, Tab::Dashboard, "📊  Dashboard");
            ui.selectable_value(&mut state.selected_tab, Tab::Devices, "📱  Devices");
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
        egui::CentralPanel::default().show(ctx, |ui| {
            match active_tab {
                Tab::Dashboard => self.show_dashboard(ui, server_running),
                Tab::Devices => self.show_devices(ui),
                Tab::Settings => self.show_settings(ui, frame),
                Tab::About => self.show_about(ui),
            }
        });
    }
}

impl DesktopApp {
    fn show_dashboard(&self, ui: &mut egui::Ui, server_running: bool) {
        ui.heading("Dashboard");
        ui.separator();

        Frame::group(ui.style()).show(ui, |ui| {
            ui.label(RichText::new("Status").strong());
            let status = if server_running { "Running" } else { "Stopped" };
            ui.label(format!("Server: {}", status));
            ui.label(format!("Port: {}", crate::agent::PORT));
            ui.label(format!("Data dir: {}", device_store::get_data_dir().display()));
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

        let devices = device_store::load_devices();
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
                                device_store::forget_device(&device.device_id);
                                info!("Forgot device: {} ({})", device.device_name, device.device_id);
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

    fn show_settings(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
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
