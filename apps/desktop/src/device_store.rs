use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TrustedDevice {
    pub device_id: String,
    pub device_name: String,
    pub last_seen: u64,
    pub paired_at: u64,
}

pub fn get_data_dir() -> PathBuf {
    std::env::var("APPDATA")
        .map(|p| PathBuf::from(p).join("AutoMatDeck"))
        .unwrap_or_else(|_| PathBuf::from("data"))
}

fn store_path() -> PathBuf {
    get_data_dir().join("trusted_devices.json")
}

pub fn load_devices() -> Vec<TrustedDevice> {
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

pub fn is_trusted(device_id: &str) -> bool {
    load_devices().iter().any(|d| d.device_id == device_id)
}

pub fn add_device(device_id: &str, device_name: &str) {
    let mut devices = load_devices();
    devices.retain(|d| d.device_id != device_id);
    let now = now_secs();
    devices.push(TrustedDevice {
        device_id: device_id.to_string(),
        device_name: device_name.to_string(),
        last_seen: now,
        paired_at: now,
    });
    save_devices(&devices);
}

pub fn touch_device(device_id: &str) {
    let mut devices = load_devices();
    if let Some(d) = devices.iter_mut().find(|d| d.device_id == device_id) {
        d.last_seen = now_secs();
        save_devices(&devices);
    }
}

pub fn forget_device(device_id: &str) {
    let mut devices = load_devices();
    devices.retain(|d| d.device_id != device_id);
    save_devices(&devices);
}

pub fn rename_device(device_id: &str, new_name: &str) {
    let mut devices = load_devices();
    if let Some(d) = devices.iter_mut().find(|d| d.device_id == device_id) {
        d.device_name = new_name.to_string();
        save_devices(&devices);
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
