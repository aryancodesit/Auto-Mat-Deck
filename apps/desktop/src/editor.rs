use egui::RichText;

use crate::command::Command;
use crate::command::CommandError;
use crate::model::*;

/// Parent-scoped editing context for button property drafts.
/// ButtonId alone is insufficient under ADR-007.
#[derive(Clone, PartialEq, Eq)]
struct EditingButtonContext {
    profile_id: ProfileId,
    page_id: PageId,
    button_id: ButtonId,
}

/// Parent-scoped move state — ensures destination PageId is valid only
/// within the source context.
#[derive(Clone, PartialEq, Eq)]
struct MoveButtonContext {
    profile_id: ProfileId,
    source_page_id: PageId,
    button_id: ButtonId,
}

#[derive(Clone)]
pub struct EditorUi {
    // Selection — stable IDs
    pub selected_profile_id: Option<ProfileId>,
    pub selected_page_id: Option<PageId>,
    pub selected_button_id: Option<ButtonId>,

    // Draft buffers
    create_profile_name: String,
    edit_profile_name: String,
    create_page_name: String,
    edit_page_name: String,

    // Modal flags
    show_create_profile: bool,
    show_rename_profile: bool,
    show_delete_profile_confirm: bool,
    show_create_page: bool,
    show_rename_page: bool,
    show_delete_page_confirm: bool,

    // Button properties drafts (parent-scoped, never ButtonId alone)
    editing_button_context: Option<EditingButtonContext>,
    edit_button_label: String,
    edit_button_action: String,
    edit_button_payload: String,

    // ponytail: action-specific draft fields replace raw JSON as primary UX.
    // Raw JSON remains available as edit_button_payload for round-trip.
    draft_launch_app: String,
    draft_url: String,
    draft_file_path: String,
    draft_notify_title: String,
    draft_notify_body: String,

    // Move state (parent-scoped, never PageId alone)
    move_button_context: Option<MoveButtonContext>,
    move_target_page_id: Option<PageId>,

    // Feedback
    pub last_command_error: Option<CommandError>,
    button_validation_error: Option<String>,
}

impl EditorUi {
    pub fn new() -> Self {
        Self {
            selected_profile_id: None,
            selected_page_id: None,
            selected_button_id: None,
            create_profile_name: String::new(),
            edit_profile_name: String::new(),
            create_page_name: String::new(),
            edit_page_name: String::new(),
            show_create_profile: false,
            show_rename_profile: false,
            show_delete_profile_confirm: false,
            show_create_page: false,
            show_rename_page: false,
            show_delete_page_confirm: false,
            editing_button_context: None,
            edit_button_label: String::new(),
            edit_button_action: String::new(),
            edit_button_payload: String::new(),
            draft_launch_app: String::new(),
            draft_url: String::new(),
            draft_file_path: String::new(),
            draft_notify_title: String::new(),
            draft_notify_body: String::new(),
            move_button_context: None,
            move_target_page_id: None,
            last_command_error: None,
            button_validation_error: None,
        }
    }

    /// Ensure ephemeral selection is valid against the current Document.
    /// This mutates EditorUi state only — never Document.
    pub fn reconcile(&mut self, doc: &Document) {
        if let Some(ref pid) = self.selected_profile_id.clone() {
            if !doc.profiles.iter().any(|p| p.id == *pid) {
                self.clear_profile_selection();
            }
        }

        if self.selected_profile_id.is_none() {
            if let Some(first) = doc.profiles.first() {
                self.selected_profile_id = Some(first.id.clone());
                self.selected_page_id = None;
                self.selected_button_id = None;
            }
        }

        if let Some(ref pid) = self.selected_profile_id.clone() {
            if let Some(ref pgid) = self.selected_page_id {
                let page_valid = doc
                    .profiles
                    .iter()
                    .find(|p| p.id == *pid)
                    .map_or(false, |p| p.pages.iter().any(|pg| pg.id == *pgid));
                if !page_valid {
                    self.selected_page_id = None;
                    self.selected_button_id = None;
                }
            }

            if self.selected_page_id.is_none() {
                if let Some(p) = doc.profiles.iter().find(|p| p.id == *pid) {
                    if let Some(first) = p.pages.first() {
                        self.selected_page_id = Some(first.id.clone());
                    }
                }
                self.selected_button_id = None;
            }
        } else {
            self.selected_page_id = None;
            self.selected_button_id = None;
        }

        // ButtonId validation — parent-scoped, never global
        if self.selected_button_id.is_some() {
            let button_valid = self
                .selected_profile_id
                .as_ref()
                .and_then(|pid| doc.profiles.iter().find(|p| p.id == *pid))
                .and_then(|p| {
                    self.selected_page_id
                        .as_ref()
                        .and_then(|pgid| p.pages.iter().find(|pg| pg.id == *pgid))
                })
                .map_or(false, |pg| {
                    pg.buttons
                        .iter()
                        .any(|b| Some(&b.id) == self.selected_button_id.as_ref())
                });
            if !button_valid {
                self.selected_button_id = None;
            }
        }
    }

    // ── Selection helpers ──

    pub fn select_profile(&mut self, id: ProfileId) {
        self.selected_profile_id = Some(id);
        self.selected_page_id = None;
        self.selected_button_id = None;
    }

    pub fn select_page(&mut self, id: PageId) {
        self.selected_page_id = Some(id);
        self.selected_button_id = None;
    }

    fn clear_profile_selection(&mut self) {
        self.selected_profile_id = None;
        self.selected_page_id = None;
        self.selected_button_id = None;
    }

    /// Resolve selected Button through parent-scoped chain. Read-only.
    fn resolved_button<'a>(&self, doc: &'a Document) -> Option<&'a Button> {
        self.selected_profile_id
            .as_ref()
            .and_then(|pid| doc.profiles.iter().find(|p| p.id == *pid))
            .and_then(|p| {
                self.selected_page_id
                    .as_ref()
                    .and_then(|pgid| p.pages.iter().find(|pg| pg.id == *pgid))
            })
            .and_then(|pg| {
                self.selected_button_id
                    .as_ref()
                    .and_then(|bid| pg.buttons.iter().find(|b| &b.id == bid))
            })
    }

    /// Build the current selected editing context if all three IDs exist.
    fn current_editing_context(&self) -> Option<EditingButtonContext> {
        match (
            self.selected_profile_id.clone(),
            self.selected_page_id.clone(),
            self.selected_button_id.clone(),
        ) {
            (Some(profile_id), Some(page_id), Some(button_id)) => Some(EditingButtonContext {
                profile_id,
                page_id,
                button_id,
            }),
            _ => None,
        }
    }

    /// Synchronize button draft buffers when editing context changes.
    /// Uses parent-scoped identity — ButtonId alone is insufficient.
    fn sync_button_drafts(&mut self, doc: &Document) {
        let current_ctx = self.current_editing_context();
        if self.editing_button_context != current_ctx {
            if let Some(ref ctx) = current_ctx {
                if let Some(btn) = self.resolved_button(doc) {
                    self.editing_button_context = Some(ctx.clone());
                    self.edit_button_label = btn.label.clone();
                    self.edit_button_action = btn.action.action_name.clone();
                    self.edit_button_payload = btn.action.payload.to_string();
                    self.hydrate_action_drafts(&btn.action.action_name, &btn.action.payload);
                    self.button_validation_error = None;
                    return;
                }
            }
            self.editing_button_context = None;
            self.edit_button_label.clear();
            self.edit_button_action.clear();
            self.edit_button_payload.clear();
            self.clear_action_drafts();
            self.button_validation_error = None;
        }
    }

    /// Build the current move context if all three source IDs exist.
    fn current_move_context(&self) -> Option<MoveButtonContext> {
        match (
            self.selected_profile_id.clone(),
            self.selected_page_id.clone(),
            self.selected_button_id.clone(),
        ) {
            (Some(profile_id), Some(source_page_id), Some(button_id)) => Some(MoveButtonContext {
                profile_id,
                source_page_id,
                button_id,
            }),
            _ => None,
        }
    }

    /// Reconcile move destination state against the current source context.
    /// Clears stale targets from previous contexts.
    fn sync_move_state(&mut self, doc: &Document) {
        let current_ctx = self.current_move_context();
        if self.move_button_context != current_ctx {
            // Source context changed — reset target
            self.move_button_context = current_ctx.clone();
            self.move_target_page_id = None;
        }
        if let Some(ref ctx) = self.move_button_context {
            // Validate that selected profile still exists
            let profile = match doc.profiles.iter().find(|p| p.id == ctx.profile_id) {
                Some(p) => p,
                None => {
                    self.move_button_context = None;
                    self.move_target_page_id = None;
                    return;
                }
            };
            // Validate current target (if any) belongs to profile and is not source
            if let Some(ref tgt) = self.move_target_page_id {
                let valid =
                    profile.pages.iter().any(|pg| pg.id == *tgt) && *tgt != ctx.source_page_id;
                if !valid {
                    self.move_target_page_id = None;
                }
            }
        } else {
            self.move_target_page_id = None;
        }
    }

    // ── Command construction helpers (private, testable) ──

    /// Clear all action-specific draft fields.
    fn clear_action_drafts(&mut self) {
        self.draft_launch_app.clear();
        self.draft_url.clear();
        self.draft_file_path.clear();
        self.draft_notify_title.clear();
        self.draft_notify_body.clear();
    }

    /// Hydrate action-specific draft fields from an existing payload.
    fn hydrate_action_drafts(&mut self, action: &str, payload: &serde_json::Value) {
        self.clear_action_drafts();
        match action {
            "launch" => {
                if let Some(app) = payload.get("app").and_then(|v| v.as_str()) {
                    self.draft_launch_app = app.to_string();
                }
            }
            "open_url" => {
                if let Some(url) = payload.get("url").and_then(|v| v.as_str()) {
                    self.draft_url = url.to_string();
                }
            }
            "open_file" => {
                if let Some(path) = payload.get("path").and_then(|v| v.as_str()) {
                    self.draft_file_path = path.to_string();
                }
            }
            "notify" => {
                if let Some(title) = payload.get("title").and_then(|v| v.as_str()) {
                    self.draft_notify_title = title.to_string();
                }
                if let Some(body) = payload.get("body").and_then(|v| v.as_str()) {
                    self.draft_notify_body = body.to_string();
                }
            }
            _ => {}
        }
    }

    /// Build UpdateButton from current typed drafts and selected context.
    fn build_update_button_command(&mut self) -> Option<Command> {
        let (pid, pgid, bid) = match (
            self.selected_profile_id.clone(),
            self.selected_page_id.clone(),
            self.selected_button_id.clone(),
        ) {
            (Some(pid), Some(pgid), Some(bid)) => (pid, pgid, bid),
            _ => return None,
        };
        let payload = self.build_payload_from_drafts();
        self.button_validation_error = None;
        Some(Command::UpdateButton {
            profile_id: pid,
            page_id: pgid,
            button: Button {
                id: bid,
                label: self.edit_button_label.clone(),
                action: ActionReference {
                    action_name: self.edit_button_action.clone(),
                    payload,
                },
            },
        })
    }

    /// Build serde_json::Value payload from typed draft fields based on selected action.
    fn build_payload_from_drafts(&self) -> serde_json::Value {
        match self.edit_button_action.as_str() {
            "launch" => serde_json::json!({"app": self.draft_launch_app}),
            "open_url" => serde_json::json!({"url": self.draft_url}),
            "open_file" => serde_json::json!({"path": self.draft_file_path}),
            "lock" => serde_json::json!({}),
            "notify" => serde_json::json!({
                "title": self.draft_notify_title,
                "body": self.draft_notify_body
            }),
            _ => serde_json::Value::Null,
        }
    }

    /// Build RemoveButton from selected context.
    /// Does not clear selection or drafts — that's the caller's responsibility
    /// after successful dispatch.
    fn build_remove_button_command(&self) -> Option<Command> {
        match (
            self.selected_profile_id.clone(),
            self.selected_page_id.clone(),
            self.selected_button_id.clone(),
        ) {
            (Some(pid), Some(pgid), Some(bid)) => Some(Command::RemoveButton {
                profile_id: pid,
                page_id: pgid,
                button_id: bid,
            }),
            _ => None,
        }
    }

    /// Build MoveButton from current move state.
    /// Re-validates target parent scope against the Document for defense in depth.
    /// Does not clear selection, does not switch pages, does not dispatch.
    fn build_move_button_command(&self, doc: &Document) -> Option<Command> {
        let ctx = self.move_button_context.as_ref()?;
        let tgt = self.move_target_page_id.as_ref()?;
        // Validate that the target page exists in the correct profile, and is not source
        let profile = doc.profiles.iter().find(|p| p.id == ctx.profile_id)?;
        let target_valid =
            profile.pages.iter().any(|pg| pg.id == *tgt) && *tgt != ctx.source_page_id;
        if !target_valid {
            return None;
        }
        Some(Command::MoveButton {
            profile_id: ctx.profile_id.clone(),
            from_page: ctx.source_page_id.clone(),
            button_id: ctx.button_id.clone(),
            to_page: tgt.clone(),
        })
    }

    // ── Editor rendering — returns the Command to dispatch if user took an action ──

    pub fn show(&mut self, ui: &mut egui::Ui, doc: &Document) -> Option<Command> {
        self.reconcile(doc);
        self.sync_button_drafts(doc);
        self.sync_move_state(doc);

        ui.heading("Editor");
        ui.separator();

        // ── Error bar ──
        if let Some(ref err) = self.last_command_error {
            ui.colored_label(egui::Color32::RED, format!("{:?}", err));
            ui.add_space(4.0);
        }

        // ── Profile selector + actions ──
        ui.horizontal(|ui| {
            ui.label(RichText::new("Profile:").strong());
            for p in &doc.profiles {
                let selected = self
                    .selected_profile_id
                    .as_ref()
                    .map_or(false, |sid| sid == &p.id);
                if ui.selectable_label(selected, &p.name).clicked() {
                    self.select_profile(p.id.clone());
                }
            }
            if ui.button("+").clicked() {
                self.show_create_profile = true;
                self.create_profile_name.clear();
            }
        });

        if let Some(ref pid) = self.selected_profile_id.clone() {
            if doc.profiles.iter().any(|p| p.id == *pid) {
                ui.horizontal(|ui| {
                    if ui.button("Rename").clicked() {
                        if let Some(p) = doc.profiles.iter().find(|p| p.id == *pid) {
                            self.edit_profile_name = p.name.clone();
                            self.show_rename_profile = true;
                        }
                    }
                    if ui.button("Delete").clicked() {
                        self.show_delete_profile_confirm = true;
                    }
                });
            }
        }

        // ── Create Profile modal ──
        if self.show_create_profile {
            let mut cmd: Option<Command> = None;
            ui.group(|ui| {
                ui.label("Create Profile");
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut self.create_profile_name);
                });
                let valid = !self.create_profile_name.is_empty();
                if ui.add_enabled(valid, egui::Button::new("Create")).clicked() {
                    let pid = ProfileId::new();
                    let pgid = PageId::new();
                    let name = self.create_profile_name.clone();
                    self.select_profile(pid.clone());
                    self.show_create_profile = false;
                    self.create_profile_name.clear();
                    cmd = Some(Command::CreateProfile {
                        profile_id: pid,
                        initial_page_id: pgid,
                        name,
                    });
                }
                if ui.button("Cancel").clicked() {
                    self.show_create_profile = false;
                }
            });
            if let Some(c) = cmd {
                return Some(c);
            }
        }

        // ── Rename Profile modal ──
        if self.show_rename_profile {
            let mut cmd: Option<Command> = None;
            ui.group(|ui| {
                ui.label("Rename Profile");
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut self.edit_profile_name);
                });
                let valid = !self.edit_profile_name.is_empty();
                if ui.add_enabled(valid, egui::Button::new("Rename")).clicked() {
                    if let Some(ref pid) = self.selected_profile_id.clone() {
                        let new_name = self.edit_profile_name.clone();
                        self.show_rename_profile = false;
                        cmd = Some(Command::RenameProfile {
                            profile_id: pid.clone(),
                            new_name,
                        });
                    }
                }
                if ui.button("Cancel").clicked() {
                    self.show_rename_profile = false;
                }
            });
            if let Some(c) = cmd {
                return Some(c);
            }
        }

        // ── Delete Profile confirmation ──
        if self.show_delete_profile_confirm {
            let mut cmd: Option<Command> = None;
            ui.group(|ui| {
                ui.label("Delete this profile?");
                ui.horizontal(|ui| {
                    if ui.button("Delete").clicked() {
                        if let Some(ref pid) = self.selected_profile_id.clone() {
                            self.show_delete_profile_confirm = false;
                            cmd = Some(Command::DeleteProfile {
                                profile_id: pid.clone(),
                            });
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        self.show_delete_profile_confirm = false;
                    }
                });
            });
            if let Some(c) = cmd {
                return Some(c);
            }
        }

        // ── Page selector + actions ──
        if let Some(ref pid) = self.selected_profile_id.clone() {
            if let Some(p) = doc.profiles.iter().find(|p| p.id == *pid) {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Page:").strong());
                    for pg in &p.pages {
                        let selected = self
                            .selected_page_id
                            .as_ref()
                            .map_or(false, |spg| spg == &pg.id);
                        if ui.selectable_label(selected, &pg.name).clicked() {
                            self.select_page(pg.id.clone());
                        }
                    }
                    if ui.button("+").clicked() {
                        self.show_create_page = true;
                        self.create_page_name.clear();
                    }
                });

                if let Some(ref pgid) = self.selected_page_id.clone() {
                    if p.pages.iter().any(|pg| pg.id == *pgid) {
                        ui.horizontal(|ui| {
                            if ui.button("Rename").clicked() {
                                if let Some(pg) = p.pages.iter().find(|pg| pg.id == *pgid) {
                                    self.edit_page_name = pg.name.clone();
                                    self.show_rename_page = true;
                                }
                            }
                            if ui.button("Delete").clicked() {
                                self.show_delete_page_confirm = true;
                            }
                        });
                    }
                }

                // ── Create Page modal ──
                if self.show_create_page {
                    let mut cmd: Option<Command> = None;
                    ui.group(|ui| {
                        ui.label("Add Page");
                        ui.horizontal(|ui| {
                            ui.label("Name:");
                            ui.text_edit_singleline(&mut self.create_page_name);
                        });
                        let valid = !self.create_page_name.is_empty();
                        if ui.add_enabled(valid, egui::Button::new("Add")).clicked() {
                            let pgid = PageId::new();
                            let name = self.create_page_name.clone();
                            self.select_page(pgid.clone());
                            self.show_create_page = false;
                            self.create_page_name.clear();
                            cmd = Some(Command::AddPage {
                                profile_id: pid.clone(),
                                page_id: pgid,
                                name,
                            });
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_create_page = false;
                        }
                    });
                    if let Some(c) = cmd {
                        return Some(c);
                    }
                }

                // ── Rename Page modal ──
                if self.show_rename_page {
                    let mut cmd: Option<Command> = None;
                    ui.group(|ui| {
                        ui.label("Rename Page");
                        ui.horizontal(|ui| {
                            ui.label("Name:");
                            ui.text_edit_singleline(&mut self.edit_page_name);
                        });
                        let valid = !self.edit_page_name.is_empty();
                        if ui.add_enabled(valid, egui::Button::new("Rename")).clicked() {
                            if let Some(ref pgid) = self.selected_page_id.clone() {
                                let new_name = self.edit_page_name.clone();
                                self.show_rename_page = false;
                                cmd = Some(Command::RenamePage {
                                    profile_id: pid.clone(),
                                    page_id: pgid.clone(),
                                    new_name,
                                });
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_rename_page = false;
                        }
                    });
                    if let Some(c) = cmd {
                        return Some(c);
                    }
                }

                // ── Delete Page confirmation ──
                if self.show_delete_page_confirm {
                    let mut cmd: Option<Command> = None;
                    ui.group(|ui| {
                        ui.label("Delete this page?");
                        ui.horizontal(|ui| {
                            if ui.button("Delete").clicked() {
                                if let Some(ref pgid) = self.selected_page_id.clone() {
                                    self.show_delete_page_confirm = false;
                                    cmd = Some(Command::DeletePage {
                                        profile_id: pid.clone(),
                                        page_id: pgid.clone(),
                                    });
                                }
                            }
                            if ui.button("Cancel").clicked() {
                                self.show_delete_page_confirm = false;
                            }
                        });
                    });
                    if let Some(c) = cmd {
                        return Some(c);
                    }
                }

                // ── Button grid for selected page ──
                if let Some(ref pgid) = self.selected_page_id.clone() {
                    if let Some(pg) = p.pages.iter().find(|pg| pg.id == *pgid) {
                        ui.add_space(8.0);
                        ui.label(RichText::new("Buttons").strong());

                        let mut clicked_add = false;

                        egui::Grid::new("button_grid")
                            .striped(true)
                            .min_col_width(80.0)
                            .show(ui, |ui| {
                                for (i, btn) in pg.buttons.iter().enumerate() {
                                    let selected = self
                                        .selected_button_id
                                        .as_ref()
                                        .map_or(false, |b| b == &btn.id);
                                    if ui.selectable_label(selected, &btn.label).clicked() {
                                        self.selected_button_id = Some(btn.id.clone());
                                    }
                                    if i > 0 && i % 4 == 3 {
                                        ui.end_row();
                                    }
                                }
                            });

                        ui.horizontal(|ui| {
                            if ui.button("+ Add Button").clicked() {
                                clicked_add = true;
                            }
                        });

                        if clicked_add {
                            let bid = ButtonId::new();
                            let btn = Button {
                                id: bid.clone(),
                                label: "New".into(),
                                action: ActionReference {
                                    action_name: String::new(),
                                    payload: serde_json::Value::Null,
                                },
                            };
                            self.selected_button_id = Some(bid);
                            return Some(Command::AddButton {
                                profile_id: pid.clone(),
                                page_id: pgid.clone(),
                                button: btn,
                            });
                        }

                        // ── Button Properties panel ──
                        if self.selected_button_id.is_some() {
                            ui.add_space(8.0);
                            ui.separator();
                            ui.label(RichText::new("Properties").strong());

                            if let Some(ref err) = self.button_validation_error {
                                ui.colored_label(egui::Color32::YELLOW, err);
                            }

                            ui.horizontal(|ui| {
                                ui.label("Label:");
                                ui.text_edit_singleline(&mut self.edit_button_label);
                            });

                            let prev_action = self.edit_button_action.clone();
                            ui.horizontal(|ui| {
                                ui.label("Action:");
                                egui::ComboBox::from_id_salt("action_combo")
                                    .selected_text(&self.edit_button_action)
                                    .show_ui(ui, |ui| {
                                        let names =
                                            ["launch", "open_url", "open_file", "lock", "notify"];
                                        for name in &names {
                                            ui.selectable_value(
                                                &mut self.edit_button_action,
                                                name.to_string(),
                                                *name,
                                            );
                                        }
                                    });
                            });

                            // Clear action drafts when action selection changes
                            if self.edit_button_action != prev_action {
                                self.clear_action_drafts();
                            }

                            // Render action-specific form fields
                            match self.edit_button_action.as_str() {
                                "launch" => {
                                    ui.horizontal(|ui| {
                                        ui.label("App:");
                                        ui.text_edit_singleline(&mut self.draft_launch_app);
                                    });
                                    ui.horizontal(|ui| {
                                        if ui.button("Browse...").clicked() {
                                            if let Some(path) = rfd::FileDialog::new()
                                                .set_title("Select Application")
                                                .pick_file()
                                            {
                                                self.draft_launch_app = path.display().to_string();
                                            }
                                        }
                                    });
                                }
                                "open_url" => {
                                    ui.horizontal(|ui| {
                                        ui.label("URL:");
                                        ui.text_edit_singleline(&mut self.draft_url);
                                    });
                                }
                                "open_file" => {
                                    ui.horizontal(|ui| {
                                        ui.label("Path:");
                                        ui.text_edit_singleline(&mut self.draft_file_path);
                                    });
                                    ui.horizontal(|ui| {
                                        if ui.button("Choose File...").clicked() {
                                            if let Some(path) = rfd::FileDialog::new()
                                                .set_title("Select File")
                                                .pick_file()
                                            {
                                                self.draft_file_path = path.display().to_string();
                                            }
                                        }
                                    });
                                }
                                "notify" => {
                                    ui.horizontal(|ui| {
                                        ui.label("Title:");
                                        ui.text_edit_singleline(&mut self.draft_notify_title);
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Message:");
                                        ui.text_edit_singleline(&mut self.draft_notify_body);
                                    });
                                }
                                "lock" => {
                                    ui.label("(No additional settings needed)");
                                }
                                _ => {}
                            }

                            let mut save_cmd: Option<Command> = None;
                            let mut remove_cmd: Option<Command> = None;
                            let mut move_cmd: Option<Command> = None;
                            ui.horizontal(|ui| {
                                if ui.button("Save").clicked() {
                                    save_cmd = self.build_update_button_command();
                                }
                                if ui.button("Remove").clicked() {
                                    remove_cmd = self.build_remove_button_command();
                                }
                            });
                            if let Some(c) = save_cmd {
                                return Some(c);
                            }
                            if let Some(c) = remove_cmd {
                                return Some(c);
                            }

                            // ── Move Button ──
                            if self.move_button_context.is_some() {
                                ui.add_space(4.0);
                                ui.separator();
                                ui.label(RichText::new("Move").strong());

                                // Build page list: pages in selected profile, excluding source
                                let profile = self
                                    .selected_profile_id
                                    .as_ref()
                                    .and_then(|pid| doc.profiles.iter().find(|p| p.id == *pid));
                                if let Some(profile) = profile {
                                    let source_page = self.selected_page_id.as_ref();
                                    let candidate_pages: Vec<&Page> = profile
                                        .pages
                                        .iter()
                                        .filter(|pg| source_page.map_or(true, |sp| pg.id != *sp))
                                        .collect();

                                    if candidate_pages.is_empty() {
                                        ui.colored_label(
                                            egui::Color32::GRAY,
                                            "No other pages available",
                                        );
                                    } else {
                                        ui.horizontal(|ui| {
                                            ui.label("Move to:");
                                            let display = self
                                                .move_target_page_id
                                                .as_ref()
                                                .and_then(|tgt| {
                                                    candidate_pages
                                                        .iter()
                                                        .find(|pg| pg.id == *tgt)
                                                        .map(|pg| pg.name.as_str())
                                                })
                                                .unwrap_or("");

                                            egui::ComboBox::from_id_salt("move_target")
                                                .selected_text(display)
                                                .show_ui(ui, |ui| {
                                                    for pg in &candidate_pages {
                                                        let selected = self
                                                            .move_target_page_id
                                                            .as_ref()
                                                            .map_or(false, |t| t == &pg.id);
                                                        let label = pg.name.clone();
                                                        if ui
                                                            .selectable_label(selected, label)
                                                            .clicked()
                                                        {
                                                            self.move_target_page_id =
                                                                Some(pg.id.clone());
                                                        }
                                                    }
                                                });
                                        });

                                        if self.move_target_page_id.is_some() {
                                            if ui.button("Move").clicked() {
                                                move_cmd = self.build_move_button_command(doc);
                                            }
                                        }
                                    }
                                }
                            }
                            if let Some(c) = move_cmd {
                                return Some(c);
                            }
                        }
                    }
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_doc(
        profile_count: usize,
        pages_per_profile: usize,
        buttons_per_page: usize,
    ) -> Document {
        let mut doc = Document::empty();
        doc.profiles = (0..profile_count)
            .map(|pi| Profile {
                id: ProfileId::from_string(format!("profile-{}", pi)),
                name: format!("Profile {}", pi),
                pages: (0..pages_per_profile)
                    .map(|gi| Page {
                        id: PageId::from_string(format!("page-{}-{}", pi, gi)),
                        name: format!("Page {}", gi),
                        buttons: (0..buttons_per_page)
                            .map(|bi| Button {
                                id: ButtonId::from_string(format!("btn-{}-{}-{}", pi, gi, bi)),
                                label: format!("Btn {}", bi),
                                action: ActionReference {
                                    action_name: String::new(),
                                    payload: serde_json::json!({"key": bi}),
                                },
                            })
                            .collect(),
                    })
                    .collect(),
            })
            .collect();
        doc
    }

    /// Build a doc with two profiles, each having a button with the same ButtonId string
    /// but with different labels/actions/payloads.
    fn cross_context_doc() -> Document {
        let shared_bid = ButtonId::from_string("shared-btn");
        let mut doc = Document::empty();
        doc.profiles = vec![
            Profile {
                id: ProfileId::from_string("profile-a"),
                name: "Profile A".into(),
                pages: vec![Page {
                    id: PageId::from_string("page-a"),
                    name: "Page A".into(),
                    buttons: vec![Button {
                        id: shared_bid.clone(),
                        label: "A Label".into(),
                        action: ActionReference {
                            action_name: "launch".into(),
                            payload: serde_json::json!({"from": "a"}),
                        },
                    }],
                }],
            },
            Profile {
                id: ProfileId::from_string("profile-b"),
                name: "Profile B".into(),
                pages: vec![Page {
                    id: PageId::from_string("page-b"),
                    name: "Page B".into(),
                    buttons: vec![Button {
                        id: shared_bid,
                        label: "B Label".into(),
                        action: ActionReference {
                            action_name: "lock".into(),
                            payload: serde_json::json!({"from": "b"}),
                        },
                    }],
                }],
            },
        ];
        doc
    }

    // ── Basic reconciliation tests ──

    #[test]
    fn no_selected_profile_selects_first() {
        let doc = test_doc(2, 1, 0);
        let mut ui = EditorUi::new();
        assert!(ui.selected_profile_id.is_none());
        ui.reconcile(&doc);
        assert_eq!(ui.selected_profile_id.unwrap().as_str(), "profile-0");
    }

    #[test]
    fn missing_selected_profile_falls_back_to_first() {
        let doc = test_doc(2, 1, 0);
        let mut ui = EditorUi::new();
        ui.selected_profile_id = Some(ProfileId::from_string("nonexistent"));
        ui.reconcile(&doc);
        assert_eq!(ui.selected_profile_id.unwrap().as_str(), "profile-0");
    }

    #[test]
    fn missing_page_selects_first_in_profile() {
        let doc = test_doc(1, 2, 0);
        let mut ui = EditorUi::new();
        ui.reconcile(&doc);
        ui.selected_page_id = Some(PageId::from_string("nope"));
        ui.reconcile(&doc);
        assert_eq!(ui.selected_page_id.unwrap().as_str(), "page-0-0");
    }

    #[test]
    fn error_persists_through_reconcile() {
        let mut ui = EditorUi::new();
        ui.last_command_error = Some(CommandError::ProfileNotFound);
        let doc = test_doc(1, 1, 0);
        ui.reconcile(&doc);
        assert!(ui.last_command_error.is_some());
    }

    // ── Button selection and parent-scoped reconciliation ──

    #[test]
    fn selected_button_preserved_when_valid() {
        let doc = test_doc(1, 1, 3);
        let mut ui = EditorUi::new();
        ui.reconcile(&doc);
        ui.selected_button_id = Some(ButtonId::from_string("btn-0-0-1"));
        ui.reconcile(&doc);
        assert_eq!(ui.selected_button_id.unwrap().as_str(), "btn-0-0-1");
    }

    #[test]
    fn missing_button_id_cleared() {
        let doc = test_doc(1, 1, 2);
        let mut ui = EditorUi::new();
        ui.reconcile(&doc);
        ui.selected_button_id = Some(ButtonId::from_string("nope"));
        ui.reconcile(&doc);
        assert!(ui.selected_button_id.is_none());
    }

    #[test]
    fn button_id_in_other_profile_does_not_validate_current_selection() {
        let mut doc = test_doc(2, 1, 1);
        doc.profiles[0].pages[0].buttons[0].id = ButtonId::from_string("unique-a");
        doc.profiles[1].pages[0].buttons[0].id = ButtonId::from_string("unique-b");
        let mut ui = EditorUi::new();
        ui.selected_profile_id = Some(ProfileId::from_string("profile-0"));
        ui.selected_page_id = Some(PageId::from_string("page-0-0"));
        ui.selected_button_id = Some(ButtonId::from_string("unique-b")); // belongs to profile-1
        ui.reconcile(&doc);
        assert!(ui.selected_button_id.is_none());
    }

    // ── Draft synchronization tests (parent-scoped context) ──

    #[test]
    fn selected_button_loads_drafts() {
        let doc = test_doc(1, 1, 1);
        let mut ui = EditorUi::new();
        ui.reconcile(&doc);
        ui.selected_profile_id = Some(ProfileId::from_string("profile-0"));
        ui.selected_page_id = Some(PageId::from_string("page-0-0"));
        ui.selected_button_id = Some(ButtonId::from_string("btn-0-0-0"));
        ui.sync_button_drafts(&doc);
        let ctx = ui.editing_button_context.as_ref().unwrap();
        assert_eq!(ctx.button_id.as_str(), "btn-0-0-0");
        assert_eq!(ctx.profile_id.as_str(), "profile-0");
        assert_eq!(ctx.page_id.as_str(), "page-0-0");
        assert_eq!(ui.edit_button_label, "Btn 0");
    }

    #[test]
    fn drafts_not_overwritten_on_repeated_sync_for_same_context() {
        let doc = test_doc(1, 1, 1);
        let mut ui = EditorUi::new();
        ui.reconcile(&doc);
        ui.selected_profile_id = Some(ProfileId::from_string("profile-0"));
        ui.selected_page_id = Some(PageId::from_string("page-0-0"));
        ui.selected_button_id = Some(ButtonId::from_string("btn-0-0-0"));
        ui.sync_button_drafts(&doc);
        ui.edit_button_label = "Modified".to_string();
        ui.sync_button_drafts(&doc); // same context — should NOT overwrite
        assert_eq!(ui.edit_button_label, "Modified");
    }

    /// Direct cross-parent transition with equal ButtonId must rehydrate drafts.
    /// This test must fail if synchronization compares ButtonId alone.
    #[test]
    fn equal_button_id_under_different_parent_rehydrates_drafts() {
        let doc = cross_context_doc();
        let mut ui = EditorUi::new();
        ui.reconcile(&doc);

        // Select context A: profile-a / page-a / shared-btn
        ui.selected_profile_id = Some(ProfileId::from_string("profile-a"));
        ui.selected_page_id = Some(PageId::from_string("page-a"));
        ui.selected_button_id = Some(ButtonId::from_string("shared-btn"));
        ui.sync_button_drafts(&doc);
        assert_eq!(ui.edit_button_label, "A Label");
        assert_eq!(ui.edit_button_action, "launch");
        assert_eq!(ui.edit_button_payload, r#"{"from":"a"}"#);

        // Directly assign context B WITHOUT intermediate None state
        ui.selected_profile_id = Some(ProfileId::from_string("profile-b"));
        ui.selected_page_id = Some(PageId::from_string("page-b"));
        ui.selected_button_id = Some(ButtonId::from_string("shared-btn"));
        ui.sync_button_drafts(&doc);

        // B's data must be loaded because the full parent context changed
        assert_eq!(ui.edit_button_label, "B Label");
        assert_eq!(ui.edit_button_action, "lock");
        assert_eq!(ui.edit_button_payload, r#"{"from":"b"}"#);
    }

    #[test]
    fn cleared_selection_clears_editing_context_and_all_drafts() {
        let doc = test_doc(1, 1, 1);
        let mut ui = EditorUi::new();
        ui.reconcile(&doc);
        ui.selected_profile_id = Some(ProfileId::from_string("profile-0"));
        ui.selected_page_id = Some(PageId::from_string("page-0-0"));
        ui.selected_button_id = Some(ButtonId::from_string("btn-0-0-0"));
        ui.sync_button_drafts(&doc);
        assert!(ui.editing_button_context.is_some());
        // Clear all selection
        ui.selected_profile_id = None;
        ui.selected_page_id = None;
        ui.selected_button_id = None;
        ui.sync_button_drafts(&doc);
        assert!(ui.editing_button_context.is_none());
        assert_eq!(ui.edit_button_label, "");
        assert_eq!(ui.edit_button_action, "");
        assert_eq!(ui.edit_button_payload, "");
    }

    // ── UpdateButton command helper tests ──

    #[test]
    fn launch_form_builds_exact_payload() {
        let doc = test_doc(1, 1, 1);
        let mut ui = EditorUi::new();
        ui.reconcile(&doc);
        ui.selected_profile_id = Some(ProfileId::from_string("profile-0"));
        ui.selected_page_id = Some(PageId::from_string("page-0-0"));
        ui.selected_button_id = Some(doc.profiles[0].pages[0].buttons[0].id.clone());
        ui.edit_button_action = "launch".to_string();
        ui.draft_launch_app = "chrome".to_string();

        let cmd = ui.build_update_button_command().expect("Expected command");
        if let Command::UpdateButton { button, .. } = cmd {
            assert_eq!(button.action.action_name, "launch");
            assert_eq!(button.action.payload, serde_json::json!({"app": "chrome"}));
        } else {
            panic!("Expected UpdateButton");
        }
    }

    #[test]
    fn open_url_form_builds_exact_payload() {
        let doc = test_doc(1, 1, 1);
        let mut ui = EditorUi::new();
        ui.reconcile(&doc);
        ui.selected_profile_id = Some(ProfileId::from_string("profile-0"));
        ui.selected_page_id = Some(PageId::from_string("page-0-0"));
        ui.selected_button_id = Some(doc.profiles[0].pages[0].buttons[0].id.clone());
        ui.edit_button_action = "open_url".to_string();
        ui.draft_url = "https://example.com".to_string();

        let cmd = ui.build_update_button_command().expect("Expected command");
        if let Command::UpdateButton { button, .. } = cmd {
            assert_eq!(button.action.action_name, "open_url");
            assert_eq!(
                button.action.payload,
                serde_json::json!({"url": "https://example.com"})
            );
        } else {
            panic!("Expected UpdateButton");
        }
    }

    #[test]
    fn open_file_form_builds_exact_payload() {
        let doc = test_doc(1, 1, 1);
        let mut ui = EditorUi::new();
        ui.reconcile(&doc);
        ui.selected_profile_id = Some(ProfileId::from_string("profile-0"));
        ui.selected_page_id = Some(PageId::from_string("page-0-0"));
        ui.selected_button_id = Some(doc.profiles[0].pages[0].buttons[0].id.clone());
        ui.edit_button_action = "open_file".to_string();
        ui.draft_file_path = "C:\\calc.exe".to_string();

        let cmd = ui.build_update_button_command().expect("Expected command");
        if let Command::UpdateButton { button, .. } = cmd {
            assert_eq!(button.action.action_name, "open_file");
            assert_eq!(
                button.action.payload,
                serde_json::json!({"path": "C:\\calc.exe"})
            );
        } else {
            panic!("Expected UpdateButton");
        }
    }

    #[test]
    fn lock_form_builds_exact_payload() {
        let doc = test_doc(1, 1, 1);
        let mut ui = EditorUi::new();
        ui.reconcile(&doc);
        ui.selected_profile_id = Some(ProfileId::from_string("profile-0"));
        ui.selected_page_id = Some(PageId::from_string("page-0-0"));
        ui.selected_button_id = Some(doc.profiles[0].pages[0].buttons[0].id.clone());
        ui.edit_button_action = "lock".to_string();

        let cmd = ui.build_update_button_command().expect("Expected command");
        if let Command::UpdateButton { button, .. } = cmd {
            assert_eq!(button.action.action_name, "lock");
            assert_eq!(button.action.payload, serde_json::json!({}));
        } else {
            panic!("Expected UpdateButton");
        }
    }

    #[test]
    fn notify_form_builds_exact_payload() {
        let doc = test_doc(1, 1, 1);
        let mut ui = EditorUi::new();
        ui.reconcile(&doc);
        ui.selected_profile_id = Some(ProfileId::from_string("profile-0"));
        ui.selected_page_id = Some(PageId::from_string("page-0-0"));
        ui.selected_button_id = Some(doc.profiles[0].pages[0].buttons[0].id.clone());
        ui.edit_button_action = "notify".to_string();
        ui.draft_notify_title = "Hello".to_string();
        ui.draft_notify_body = "World".to_string();

        let cmd = ui.build_update_button_command().expect("Expected command");
        if let Command::UpdateButton { button, .. } = cmd {
            assert_eq!(button.action.action_name, "notify");
            assert_eq!(
                button.action.payload,
                serde_json::json!({"title": "Hello", "body": "World"})
            );
        } else {
            panic!("Expected UpdateButton");
        }
    }

    #[test]
    fn existing_launch_payload_hydrates_launch_draft() {
        let mut doc = test_doc(1, 1, 1);
        let bid = doc.profiles[0].pages[0].buttons[0].id.clone();
        let btn = &mut doc.profiles[0].pages[0].buttons[0];
        btn.action = ActionReference {
            action_name: "launch".into(),
            payload: serde_json::json!({"app": "notepad.exe"}),
        };
        let mut ui = EditorUi::new();
        ui.reconcile(&doc);
        ui.selected_button_id = Some(bid);
        ui.sync_button_drafts(&doc);
        assert_eq!(ui.edit_button_action, "launch");
        assert_eq!(ui.draft_launch_app, "notepad.exe");
    }

    #[test]
    fn existing_open_url_payload_hydrates_url_draft() {
        let mut doc = test_doc(1, 1, 1);
        let bid = doc.profiles[0].pages[0].buttons[0].id.clone();
        let btn = &mut doc.profiles[0].pages[0].buttons[0];
        btn.action = ActionReference {
            action_name: "open_url".into(),
            payload: serde_json::json!({"url": "https://github.com"}),
        };
        let mut ui = EditorUi::new();
        ui.reconcile(&doc);
        ui.selected_button_id = Some(bid);
        ui.sync_button_drafts(&doc);
        assert_eq!(ui.edit_button_action, "open_url");
        assert_eq!(ui.draft_url, "https://github.com");
    }

    #[test]
    fn existing_open_file_payload_hydrates_path_draft() {
        let mut doc = test_doc(1, 1, 1);
        let bid = doc.profiles[0].pages[0].buttons[0].id.clone();
        let btn = &mut doc.profiles[0].pages[0].buttons[0];
        btn.action = ActionReference {
            action_name: "open_file".into(),
            payload: serde_json::json!({"path": "C:\\test.txt"}),
        };
        let mut ui = EditorUi::new();
        ui.reconcile(&doc);
        ui.selected_button_id = Some(bid);
        ui.sync_button_drafts(&doc);
        assert_eq!(ui.edit_button_action, "open_file");
        assert_eq!(ui.draft_file_path, "C:\\test.txt");
    }

    #[test]
    fn existing_notify_payload_hydrates_title_and_body() {
        let mut doc = test_doc(1, 1, 1);
        let bid = doc.profiles[0].pages[0].buttons[0].id.clone();
        let btn = &mut doc.profiles[0].pages[0].buttons[0];
        btn.action = ActionReference {
            action_name: "notify".into(),
            payload: serde_json::json!({"title": "Alert", "body": "Something happened"}),
        };
        let mut ui = EditorUi::new();
        ui.reconcile(&doc);
        ui.selected_button_id = Some(bid);
        ui.sync_button_drafts(&doc);
        assert_eq!(ui.edit_button_action, "notify");
        assert_eq!(ui.draft_notify_title, "Alert");
        assert_eq!(ui.draft_notify_body, "Something happened");
    }

    #[test]
    fn lock_hydrates_without_payload_fields() {
        let mut doc = test_doc(1, 1, 1);
        let bid = doc.profiles[0].pages[0].buttons[0].id.clone();
        let btn = &mut doc.profiles[0].pages[0].buttons[0];
        btn.action = ActionReference {
            action_name: "lock".into(),
            payload: serde_json::json!({}),
        };
        let mut ui = EditorUi::new();
        ui.reconcile(&doc);
        ui.selected_button_id = Some(bid);
        ui.sync_button_drafts(&doc);
        assert_eq!(ui.edit_button_action, "lock");
        assert_eq!(ui.draft_launch_app, "");
        assert_eq!(ui.draft_url, "");
    }

    #[test]
    fn changing_action_clears_incompatible_drafts() {
        let mut doc = test_doc(1, 1, 1);
        let bid = doc.profiles[0].pages[0].buttons[0].id.clone();
        let btn = &mut doc.profiles[0].pages[0].buttons[0];
        btn.action = ActionReference {
            action_name: "open_url".into(),
            payload: serde_json::json!({"url": "https://example.com"}),
        };
        let mut ui = EditorUi::new();
        ui.reconcile(&doc);
        ui.selected_button_id = Some(bid);
        ui.sync_button_drafts(&doc);
        assert_eq!(ui.draft_url, "https://example.com");

        // Simulate user changing action
        ui.edit_button_action = "launch".to_string();
        ui.clear_action_drafts();
        assert_eq!(ui.draft_url, "");
    }

    #[test]
    fn repeated_sync_preserves_user_typing() {
        let doc = test_doc(1, 1, 1);
        let mut ui = EditorUi::new();
        ui.reconcile(&doc);
        ui.selected_profile_id = Some(ProfileId::from_string("profile-0"));
        ui.selected_page_id = Some(PageId::from_string("page-0-0"));
        ui.selected_button_id = Some(ButtonId::from_string("btn-0-0-0"));
        ui.sync_button_drafts(&doc);
        // Modify draft
        ui.draft_notify_title = "User typed".to_string();
        // Sync again — same context, should NOT overwrite
        ui.sync_button_drafts(&doc);
        assert_eq!(ui.draft_notify_title, "User typed");
    }

    #[test]
    fn update_preserves_button_id() {
        let doc = test_doc(1, 1, 1);
        let mut ui = EditorUi::new();
        ui.reconcile(&doc);
        let bid = doc.profiles[0].pages[0].buttons[0].id.clone();
        ui.selected_profile_id = Some(ProfileId::from_string("profile-0"));
        ui.selected_page_id = Some(PageId::from_string("page-0-0"));
        ui.selected_button_id = Some(bid.clone());
        ui.edit_button_action = "lock".to_string();

        let cmd = ui.build_update_button_command();
        let cmd = cmd.expect("Expected command");
        if let Command::UpdateButton { button, .. } = cmd {
            assert_eq!(button.id, bid);
        } else {
            panic!("Expected UpdateButton");
        }
    }

    // ── RemoveButton command helper tests ──

    #[test]
    fn remove_helper_builds_parent_scoped_command() {
        let doc = test_doc(1, 1, 1);
        let mut ui = EditorUi::new();
        ui.reconcile(&doc);
        let pid = doc.profiles[0].id.clone();
        let pgid = doc.profiles[0].pages[0].id.clone();
        let bid = doc.profiles[0].pages[0].buttons[0].id.clone();
        ui.selected_profile_id = Some(pid.clone());
        ui.selected_page_id = Some(pgid.clone());
        ui.selected_button_id = Some(bid.clone());

        let cmd = ui.build_remove_button_command();
        let cmd = cmd.expect("Expected command");
        match cmd {
            Command::RemoveButton {
                profile_id,
                page_id,
                button_id,
            } => {
                assert_eq!(profile_id, pid);
                assert_eq!(page_id, pgid);
                assert_eq!(button_id, bid);
            }
            _ => panic!("Expected RemoveButton"),
        }
    }

    #[test]
    fn remove_helper_does_not_clear_selection() {
        let doc = test_doc(1, 1, 1);
        let mut ui = EditorUi::new();
        ui.reconcile(&doc);
        let bid = doc.profiles[0].pages[0].buttons[0].id.clone();
        ui.selected_profile_id = Some(doc.profiles[0].id.clone());
        ui.selected_page_id = Some(doc.profiles[0].pages[0].id.clone());
        ui.selected_button_id = Some(bid);

        // Call the helper; selection should survive
        let _ = ui.build_remove_button_command();
        assert!(ui.selected_button_id.is_some());
        assert!(ui.selected_profile_id.is_some());
        assert!(ui.selected_page_id.is_some());
    }

    // ── MoveButton tests ──

    /// A PageId from another profile must never validate as a move target.
    #[test]
    fn move_target_is_scoped_to_selected_profile() {
        let mut doc = test_doc(2, 2, 1);
        // Give each page a distinct PageId for clarity
        doc.profiles[0].pages[0].id = PageId::from_string("profile0-page0");
        doc.profiles[0].pages[1].id = PageId::from_string("profile0-page1");
        doc.profiles[1].pages[0].id = PageId::from_string("profile1-page0");
        doc.profiles[1].pages[1].id = PageId::from_string("profile1-page1");
        // Put a button in profile0-page0
        let bid = ButtonId::from_string("move-btn");
        doc.profiles[0].pages[0].buttons = vec![Button {
            id: bid.clone(),
            label: "Move Me".into(),
            action: ActionReference {
                action_name: "launch".into(),
                payload: serde_json::json!({}),
            },
        }];

        let mut ui = EditorUi::new();
        ui.reconcile(&doc);
        // Select profile-0 / page-0 / move-btn
        ui.selected_profile_id = Some(ProfileId::from_string("profile-0"));
        ui.selected_page_id = Some(PageId::from_string("profile0-page0"));
        ui.selected_button_id = Some(bid);
        ui.sync_move_state(&doc);
        // Set target to profile-1's page — should be invalid
        ui.move_target_page_id = Some(PageId::from_string("profile1-page0"));
        // sync should clear it (belongs to different profile)
        ui.sync_move_state(&doc);
        assert!(ui.move_target_page_id.is_none());
        // build_move_button_command should also reject it
        assert!(ui.build_move_button_command(&doc).is_none());
    }

    #[test]
    fn move_helper_builds_parent_scoped_command() {
        let doc = test_doc(1, 2, 1);
        let mut ui = EditorUi::new();
        ui.reconcile(&doc);

        let pid = doc.profiles[0].id.clone();
        let src_pgid = doc.profiles[0].pages[0].id.clone();
        let dst_pgid = doc.profiles[0].pages[1].id.clone();
        let bid = doc.profiles[0].pages[0].buttons[0].id.clone();

        ui.selected_profile_id = Some(pid.clone());
        ui.selected_page_id = Some(src_pgid.clone());
        ui.selected_button_id = Some(bid.clone());
        ui.sync_move_state(&doc);
        ui.move_target_page_id = Some(dst_pgid.clone());

        let cmd = ui.build_move_button_command(&doc);
        let cmd = cmd.expect("Expected MoveButton");
        match cmd {
            Command::MoveButton {
                profile_id,
                from_page,
                button_id,
                to_page,
            } => {
                assert_eq!(profile_id, pid);
                assert_eq!(from_page, src_pgid);
                assert_eq!(button_id, bid);
                assert_eq!(to_page, dst_pgid);
            }
            _ => panic!("Expected MoveButton"),
        }
    }

    #[test]
    fn move_helper_preserves_button_id() {
        let doc = test_doc(1, 2, 1);
        let mut ui = EditorUi::new();
        ui.reconcile(&doc);

        let bid = doc.profiles[0].pages[0].buttons[0].id.clone();
        let dst_pgid = doc.profiles[0].pages[1].id.clone();
        ui.selected_profile_id = Some(doc.profiles[0].id.clone());
        ui.selected_page_id = Some(doc.profiles[0].pages[0].id.clone());
        ui.selected_button_id = Some(bid.clone());
        ui.sync_move_state(&doc);
        // Must set target AFTER sync — sync clears it on first context init
        ui.move_target_page_id = Some(dst_pgid);

        let cmd = ui
            .build_move_button_command(&doc)
            .expect("Expected command");
        if let Command::MoveButton { button_id, .. } = cmd {
            assert_eq!(button_id, bid);
        } else {
            panic!("Expected MoveButton");
        }
    }

    #[test]
    fn move_helper_does_not_clear_selection() {
        let doc = test_doc(1, 2, 1);
        let mut ui = EditorUi::new();
        ui.reconcile(&doc);

        let pid = doc.profiles[0].id.clone();
        let pgid = doc.profiles[0].pages[0].id.clone();
        let bid = doc.profiles[0].pages[0].buttons[0].id.clone();

        ui.selected_profile_id = Some(pid.clone());
        ui.selected_page_id = Some(pgid.clone());
        ui.selected_button_id = Some(bid.clone());
        ui.move_target_page_id = Some(doc.profiles[0].pages[1].id.clone());
        ui.sync_move_state(&doc);

        let _ = ui.build_move_button_command(&doc);
        assert!(ui.selected_profile_id.is_some());
        assert!(ui.selected_page_id.is_some());
        assert!(ui.selected_button_id.is_some());
    }

    #[test]
    fn move_helper_does_not_follow_destination_page() {
        let doc = test_doc(1, 2, 1);
        let mut ui = EditorUi::new();
        ui.reconcile(&doc);

        let src_pgid = doc.profiles[0].pages[0].id.clone();
        let dst_pgid = doc.profiles[0].pages[1].id.clone();

        ui.selected_profile_id = Some(doc.profiles[0].id.clone());
        ui.selected_page_id = Some(src_pgid.clone());
        ui.selected_button_id = Some(doc.profiles[0].pages[0].buttons[0].id.clone());
        ui.move_target_page_id = Some(dst_pgid);
        ui.sync_move_state(&doc);

        let _ = ui.build_move_button_command(&doc);
        // selected_page_id must remain source
        assert_eq!(ui.selected_page_id.unwrap(), src_pgid);
    }

    #[test]
    fn stale_move_target_cleared_on_profile_context_change() {
        let mut doc = test_doc(2, 2, 1);
        // Distinct page IDs
        doc.profiles[0].pages[0].id = PageId::from_string("p0-pg0");
        doc.profiles[0].pages[1].id = PageId::from_string("p0-pg1");
        doc.profiles[1].pages[0].id = PageId::from_string("p1-pg0");
        doc.profiles[1].pages[1].id = PageId::from_string("p1-pg1");
        // Put a button in each profile
        doc.profiles[0].pages[0].buttons = vec![Button {
            id: ButtonId::from_string("btn"),
            label: "Btn".into(),
            action: ActionReference {
                action_name: String::new(),
                payload: serde_json::Value::Null,
            },
        }];
        doc.profiles[1].pages[0].buttons = vec![Button {
            id: ButtonId::from_string("btn"),
            label: "Btn".into(),
            action: ActionReference {
                action_name: String::new(),
                payload: serde_json::Value::Null,
            },
        }];

        let mut ui = EditorUi::new();
        ui.reconcile(&doc);

        // Set up Profile A context with a valid destination
        ui.selected_profile_id = Some(ProfileId::from_string("profile-0"));
        ui.selected_page_id = Some(PageId::from_string("p0-pg0"));
        ui.selected_button_id = Some(ButtonId::from_string("btn"));
        ui.sync_move_state(&doc);
        ui.move_target_page_id = Some(PageId::from_string("p0-pg1"));

        // Directly transition to Profile B WITHOUT intermediate None
        ui.selected_profile_id = Some(ProfileId::from_string("profile-1"));
        ui.selected_page_id = Some(PageId::from_string("p1-pg0"));
        ui.selected_button_id = Some(ButtonId::from_string("btn"));
        ui.sync_move_state(&doc);

        // Profile A's target (p0-pg1) must be cleared
        assert!(ui.move_target_page_id.is_none());
        // No command should be produced with Profile B's context + stale target
        assert!(ui.build_move_button_command(&doc).is_none());
    }

    #[test]
    fn no_valid_destination_builds_no_move_command() {
        let doc = test_doc(1, 1, 1); // single profile, single page — no valid target
        let mut ui = EditorUi::new();
        ui.reconcile(&doc);
        ui.selected_profile_id = Some(doc.profiles[0].id.clone());
        ui.selected_page_id = Some(doc.profiles[0].pages[0].id.clone());
        ui.selected_button_id = Some(doc.profiles[0].pages[0].buttons[0].id.clone());
        ui.sync_move_state(&doc);

        assert!(ui.move_button_context.is_some());
        assert!(ui.move_target_page_id.is_none()); // no valid target
        assert!(ui.build_move_button_command(&doc).is_none());
    }

    #[test]
    fn moved_button_selection_cleared_by_reconcile_after_document_change() {
        let doc = test_doc(1, 2, 1);
        let mut ui = EditorUi::new();
        ui.reconcile(&doc);

        let pid = doc.profiles[0].id.clone();
        let src_pgid = doc.profiles[0].pages[0].id.clone();
        let dst_pgid = doc.profiles[0].pages[1].id.clone();
        let bid = doc.profiles[0].pages[0].buttons[0].id.clone();

        ui.selected_profile_id = Some(pid.clone());
        ui.selected_page_id = Some(src_pgid.clone());
        ui.selected_button_id = Some(bid);

        // Apply MoveButton through the real reducer
        let moved = crate::command::apply(
            &doc,
            &Command::MoveButton {
                profile_id: pid.clone(),
                from_page: src_pgid.clone(),
                button_id: doc.profiles[0].pages[0].buttons[0].id.clone(),
                to_page: dst_pgid,
            },
        )
        .expect("MoveButton should succeed");

        // Reconcile against post-move document
        ui.reconcile(&moved);

        // Selection: source page stays, button cleared
        assert_eq!(ui.selected_page_id.unwrap(), src_pgid);
        assert!(ui.selected_button_id.is_none());
    }

    /// Equal PageId under another profile must not validate as a move target.
    #[test]
    fn equal_page_id_under_other_profile_never_validates_move_target() {
        let mut doc = test_doc(2, 1, 1);
        // Both profiles have a page with id "same-page"
        doc.profiles[0].pages[0].id = PageId::from_string("same-page");
        doc.profiles[0].pages[0].buttons = vec![Button {
            id: ButtonId::from_string("btn"),
            label: "Btn".into(),
            action: ActionReference {
                action_name: String::new(),
                payload: serde_json::Value::Null,
            },
        }];
        doc.profiles[1].pages[0].id = PageId::from_string("same-page");
        // profile-1 gets a second page to be a valid target
        doc.profiles[1].pages.push(Page {
            id: PageId::from_string("other-page"),
            name: "Other".into(),
            buttons: vec![],
        });

        let mut ui = EditorUi::new();
        ui.reconcile(&doc);

        // Select profile-0 / same-page / btn
        ui.selected_profile_id = Some(ProfileId::from_string("profile-0"));
        ui.selected_page_id = Some(PageId::from_string("same-page"));
        ui.selected_button_id = Some(ButtonId::from_string("btn"));
        ui.sync_move_state(&doc);

        // Set target to "same-page" which exists in profile-1 — must be invalid
        // because it is the current source page (same-page == same-page).
        // Also test: set target to "same-page" of profile-0 (which is source -> invalid).
        ui.move_target_page_id = Some(PageId::from_string("same-page"));
        ui.sync_move_state(&doc);
        // Cleared because it's the source page
        assert!(ui.move_target_page_id.is_none());

        // Now explicitly point to the equal PageId under profile-1's scope
        // by changing the source context to profile-1
        ui.selected_profile_id = Some(ProfileId::from_string("profile-1"));
        ui.selected_page_id = Some(PageId::from_string("same-page"));
        ui.selected_button_id = Some(ButtonId::from_string("btn"));
        ui.sync_move_state(&doc);
        // Set target to "other-page" which exists in profile-1
        ui.move_target_page_id = Some(PageId::from_string("other-page"));
        ui.sync_move_state(&doc);
        // Now the target is valid (other-page != same-page, in profile-1)
        assert_eq!(
            ui.move_target_page_id.as_ref().unwrap().as_str(),
            "other-page"
        );
        let cmd = ui.build_move_button_command(&doc);
        assert!(cmd.is_some());
        match cmd.unwrap() {
            Command::MoveButton {
                profile_id,
                to_page,
                ..
            } => {
                assert_eq!(profile_id.as_str(), "profile-1");
                assert_eq!(to_page.as_str(), "other-page");
            }
            _ => panic!("Expected MoveButton"),
        }
    }

    #[test]
    fn repeated_move_state_reconciliation_preserves_valid_target() {
        let doc = test_doc(1, 2, 1);
        let mut ui = EditorUi::new();
        ui.reconcile(&doc);

        let dst_pgid = doc.profiles[0].pages[1].id.clone();
        ui.selected_profile_id = Some(doc.profiles[0].id.clone());
        ui.selected_page_id = Some(doc.profiles[0].pages[0].id.clone());
        ui.selected_button_id = Some(doc.profiles[0].pages[0].buttons[0].id.clone());
        ui.sync_move_state(&doc);
        assert!(ui.move_target_page_id.is_none()); // no target set yet

        // Set a valid target
        ui.move_target_page_id = Some(dst_pgid.clone());

        // Repeated reconciliation must preserve it
        for _ in 0..5 {
            ui.sync_move_state(&doc);
            assert_eq!(ui.move_target_page_id.as_ref().unwrap(), &dst_pgid);
        }
    }

    /// The frozen reducer rejects same-page movement with DuplicateButtonId.
    /// The UI must not offer the source page as a valid destination.
    #[test]
    fn same_page_move_is_not_a_valid_ui_destination() {
        let doc = test_doc(1, 1, 1); // single page — no valid target
        let mut ui = EditorUi::new();
        ui.reconcile(&doc);

        ui.selected_profile_id = Some(doc.profiles[0].id.clone());
        ui.selected_page_id = Some(doc.profiles[0].pages[0].id.clone());
        ui.selected_button_id = Some(doc.profiles[0].pages[0].buttons[0].id.clone());
        ui.sync_move_state(&doc);

        // The only page available IS the source page — no valid target
        assert!(ui.move_target_page_id.is_none());
        assert!(ui.build_move_button_command(&doc).is_none());

        // Even if we manually set it to the source page, sync clears it
        ui.move_target_page_id = Some(doc.profiles[0].pages[0].id.clone());
        ui.sync_move_state(&doc);
        assert!(ui.move_target_page_id.is_none());

        // And the command helper also rejects it
        ui.move_target_page_id = Some(doc.profiles[0].pages[0].id.clone());
        assert!(ui.build_move_button_command(&doc).is_none());
    }
}
