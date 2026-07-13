use std::sync::{Arc, Mutex};

use crate::command::{self, Command, CommandError};
use crate::model::*;
use crate::repository::DocumentStore;

/// In-memory application state. The GUI reads from here, never from disk.
pub struct AppState {
    pub document: Document,
    pub server_running: bool,
    pub selected_tab: Tab,
}

#[derive(Clone, PartialEq)]
pub enum Tab {
    Dashboard,
    Devices,
    Settings,
    Pairing,
    About,
    Editor,
}

impl Default for Tab {
    fn default() -> Self {
        Tab::Dashboard
    }
}

impl AppState {
    /// Load state from the repository at startup.
    pub fn load(store: &dyn DocumentStore) -> Self {
        Self {
            document: store.load(),
            server_running: false,
            selected_tab: Tab::default(),
        }
    }

    // ── command dispatch ──

    /// Apply a domain command through the pure reducer.
    /// On success the document is replaced; on failure the state is unchanged.
    /// The caller is responsible for persistence after calling this.
    pub fn dispatch(&mut self, cmd: &Command) -> Result<(), CommandError> {
        let new_doc = command::apply(&self.document, cmd)?;
        self.document = new_doc;
        Ok(())
    }

    // ── device queries ──

    pub fn devices(&self) -> &[TrustedDevice] {
        &self.document.devices
    }

    pub fn device_count(&self) -> usize {
        self.document.devices.len()
    }

    pub fn is_trusted(&self, device_id: &str) -> bool {
        self.document
            .devices
            .iter()
            .any(|d| d.device_id.as_str() == device_id)
    }

    // ── device mutations (caller persists via store.save()) ──

    pub fn forget_device(&mut self, device_id: &str) {
        self.document
            .devices
            .retain(|d| d.device_id.as_str() != device_id);
    }

    pub fn rename_device(&mut self, device_id: &str, new_name: &str) {
        if let Some(d) = self
            .document
            .devices
            .iter_mut()
            .find(|d| d.device_id.as_str() == device_id)
        {
            d.device_name = new_name.to_string();
        }
    }

    pub fn touch_device(&mut self, device_id: &str) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        if let Some(d) = self
            .document
            .devices
            .iter_mut()
            .find(|d| d.device_id.as_str() == device_id)
        {
            d.last_seen = now;
        }
    }

    pub fn add_device(&mut self, device_id: &str, device_name: &str) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.document
            .devices
            .retain(|d| d.device_id.as_str() != device_id);
        self.document.devices.push(TrustedDevice {
            device_id: DeviceId::from_string(device_id),
            device_name: device_name.to_string(),
            last_seen: now,
            paired_at: now,
        });
    }

    // ── profile / page / button queries ──

    pub fn profiles(&self) -> &[Profile] {
        &self.document.profiles
    }

    pub fn profile_count(&self) -> usize {
        self.document.profiles.len()
    }

    // ── persistence helper ──

    pub fn persist(&self, store: &dyn DocumentStore) {
        store.save(&self.document);
    }
}

/// Shared state wrapper used across threads.
pub type SharedState = Arc<Mutex<AppState>>;

pub fn new_shared(store: &dyn DocumentStore) -> SharedState {
    Arc::new(Mutex::new(AppState::load(store)))
}
