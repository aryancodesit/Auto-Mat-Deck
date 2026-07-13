use std::path::{Path, PathBuf};

use log::{info, warn};

use crate::model::*;

// ── Trait ────────────────────────────────────────────────────

/// Abstraction over document persistence.
///
/// The GUI never calls this directly — it reads from `AppState` in memory.
/// A `JsonRepository` writes to `document.json`. Future implementations
/// could target SQLite, cloud storage, or an in-memory mock for testing.
pub trait DocumentStore: Send + Sync {
    /// Load the document. Called once at startup.
    fn load(&self) -> Document;

    /// Persist the document. Called after each mutation.
    fn save(&self, document: &Document);

    /// Path to the data directory (for display / log files).
    fn data_dir(&self) -> &Path;
}

// ── JSON implementation ──────────────────────────────────────

/// Persists the document as a single `document.json` file.
pub struct JsonRepository {
    data_dir: PathBuf,
}

impl JsonRepository {
    pub fn new() -> Self {
        let data_dir = std::env::var("APPDATA")
            .map(|p| PathBuf::from(p).join("AutoMatDeck"))
            .unwrap_or_else(|_| PathBuf::from("data"));
        std::fs::create_dir_all(&data_dir).ok();
        Self { data_dir }
    }
}

impl DocumentStore for JsonRepository {
    fn load(&self) -> Document {
        let path = self.data_dir.join("document.json");
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str::<Document>(&content) {
                    Ok(doc) => return doc,
                    Err(e) => warn!("Failed to parse document.json: {}. Falling back.", e),
                },
                Err(e) => warn!("Failed to read document.json: {}. Falling back.", e),
            }
        }

        // Attempt migration from legacy trusted_devices.json
        self.migrate_from_legacy()
    }

    fn save(&self, document: &Document) {
        let path = self.data_dir.join("document.json");
        match serde_json::to_string_pretty(document) {
            Ok(content) => {
                if let Err(e) = std::fs::write(&path, &content) {
                    warn!("Failed to write document.json: {}", e);
                } else {
                    info!(
                        "Document saved (schema v{}, {} devices, {} profiles)",
                        document.schema,
                        document.devices.len(),
                        document.profiles.len()
                    );
                }
            }
            Err(e) => warn!("Failed to serialize document: {}", e),
        }
    }

    fn data_dir(&self) -> &Path {
        &self.data_dir
    }
}

impl JsonRepository {
    /// Migrate from the legacy v0.1 trusted_devices.json format.
    fn migrate_from_legacy(&self) -> Document {
        let legacy_path = self.data_dir.join("trusted_devices.json");
        let devices = if legacy_path.exists() {
            match std::fs::read_to_string(&legacy_path) {
                Ok(content) => serde_json::from_str::<Vec<LegacyDevice>>(&content)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|d| d.into())
                    .collect(),
                Err(_) => Vec::new(),
            }
        } else {
            Vec::new()
        };

        if !devices.is_empty() {
            info!("Migrated {} devices from legacy format", devices.len());
            let backup = self.data_dir.join("trusted_devices.json.v0.1");
            let _ = std::fs::rename(&legacy_path, &backup);
        }

        let mut doc = Document::empty();
        doc.devices = devices;
        self.save(&doc);
        doc
    }
}

/// Legacy device format from v0.1 device_store.rs
#[derive(serde::Deserialize)]
struct LegacyDevice {
    device_id: String,
    device_name: String,
    last_seen: u64,
    paired_at: u64,
}

impl From<LegacyDevice> for TrustedDevice {
    fn from(d: LegacyDevice) -> Self {
        TrustedDevice {
            device_id: DeviceId::from_string(d.device_id),
            device_name: d.device_name,
            last_seen: d.last_seen,
            paired_at: d.paired_at,
        }
    }
}
