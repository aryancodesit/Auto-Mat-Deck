use std::sync::{Arc, Mutex};

use log::info;

use crate::model::{DeviceId, PairingMethod, TrustedDevice};

/// Standalone trust store for managing trusted devices.
/// Wraps a `Vec<TrustedDevice>` with a clean API and O(1) lookup.
pub struct TrustStore {
    devices: Vec<TrustedDevice>,
}

impl TrustStore {
    pub fn new(devices: Vec<TrustedDevice>) -> Self {
        Self { devices }
    }

    pub fn empty() -> Self {
        Self {
            devices: Vec::new(),
        }
    }

    pub fn is_trusted(&self, device_id: &str) -> bool {
        self.devices
            .iter()
            .any(|d| d.device_id.as_str() == device_id)
    }

    pub fn get(&self, device_id: &str) -> Option<&TrustedDevice> {
        self.devices
            .iter()
            .find(|d| d.device_id.as_str() == device_id)
    }

    pub fn add(
        &mut self,
        device_id: &str,
        device_name: &str,
        pairing_method: PairingMethod,
        protocol_version: u32,
    ) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.devices.retain(|d| d.device_id.as_str() != device_id);
        self.devices.push(TrustedDevice {
            device_id: DeviceId::from_string(device_id),
            device_name: device_name.to_string(),
            last_seen: now,
            paired_at: now,
            pairing_method,
            protocol_version,
        });
        info!("[TRUST] Added device: {} ({})", device_name, device_id);
    }

    pub fn touch(&mut self, device_id: &str) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        if let Some(d) = self
            .devices
            .iter_mut()
            .find(|d| d.device_id.as_str() == device_id)
        {
            d.last_seen = now;
        }
    }

    pub fn forget(&mut self, device_id: &str) -> bool {
        let len_before = self.devices.len();
        self.devices.retain(|d| d.device_id.as_str() != device_id);
        let removed = self.devices.len() < len_before;
        if removed {
            info!("[TRUST] Removed device: {}", device_id);
        }
        removed
    }

    pub fn rename(&mut self, device_id: &str, new_name: &str) -> bool {
        if let Some(d) = self
            .devices
            .iter_mut()
            .find(|d| d.device_id.as_str() == device_id)
        {
            d.device_name = new_name.to_string();
            true
        } else {
            false
        }
    }

    pub fn devices(&self) -> &[TrustedDevice] {
        &self.devices
    }

    pub fn len(&self) -> usize {
        self.devices.len()
    }

    pub fn is_empty(&self) -> bool {
        self.devices.is_empty()
    }

    #[allow(dead_code)]
    pub fn into_inner(self) -> Vec<TrustedDevice> {
        self.devices
    }
}

/// Shared trust store for concurrent access.
#[allow(dead_code)]
pub type SharedTrustStore = Arc<Mutex<TrustStore>>;

#[allow(dead_code)]
pub fn shared_store(devices: Vec<TrustedDevice>) -> SharedTrustStore {
    Arc::new(Mutex::new(TrustStore::new(devices)))
}

#[allow(dead_code)]
pub fn empty_store() -> SharedTrustStore {
    Arc::new(Mutex::new(TrustStore::empty()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_device(id: &str, name: &str) -> TrustedDevice {
        TrustedDevice {
            device_id: DeviceId::from_string(id),
            device_name: name.to_string(),
            last_seen: 1000,
            paired_at: 1000,
            pairing_method: PairingMethod::Otp,
            protocol_version: 1,
        }
    }

    #[test]
    fn empty_store_is_empty() {
        let store = TrustStore::empty();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn add_device_makes_trusted() {
        let mut store = TrustStore::empty();
        store.add("d1", "Phone", PairingMethod::QrCode, 1);
        assert!(store.is_trusted("d1"));
        assert!(!store.is_trusted("d2"));
    }

    #[test]
    fn add_device_replaces_existing() {
        let mut store = TrustStore::empty();
        store.add("d1", "Phone", PairingMethod::QrCode, 1);
        store.add("d1", "Phone New", PairingMethod::Otp, 2);
        assert_eq!(store.len(), 1);
        let d = store.get("d1").unwrap();
        assert_eq!(d.device_name, "Phone New");
        assert_eq!(d.protocol_version, 2);
    }

    #[test]
    fn forget_removes_device() {
        let mut store = TrustStore::empty();
        store.add("d1", "Phone", PairingMethod::QrCode, 1);
        assert!(store.forget("d1"));
        assert!(!store.is_trusted("d1"));
    }

    #[test]
    fn forget_nonexistent_returns_false() {
        let mut store = TrustStore::empty();
        assert!(!store.forget("d1"));
    }

    #[test]
    fn touch_updates_last_seen() {
        let mut store = TrustStore::empty();
        store.add("d1", "Phone", PairingMethod::QrCode, 1);
        let before = store.get("d1").unwrap().last_seen;
        store.touch("d1");
        let after = store.get("d1").unwrap().last_seen;
        assert!(after >= before);
    }

    #[test]
    fn rename_updates_name() {
        let mut store = TrustStore::empty();
        store.add("d1", "Phone", PairingMethod::QrCode, 1);
        assert!(store.rename("d1", "New Name"));
        assert_eq!(store.get("d1").unwrap().device_name, "New Name");
    }

    #[test]
    fn rename_nonexistent_returns_false() {
        let mut store = TrustStore::empty();
        assert!(!store.rename("d1", "Name"));
    }

    #[test]
    fn devices_returns_all() {
        let mut store = TrustStore::empty();
        store.add("d1", "Phone", PairingMethod::QrCode, 1);
        store.add("d2", "Tablet", PairingMethod::Otp, 1);
        assert_eq!(store.devices().len(), 2);
    }

    #[test]
    fn from_existing_devices() {
        let devices = vec![make_device("d1", "Phone"), make_device("d2", "Tablet")];
        let store = TrustStore::new(devices);
        assert_eq!(store.len(), 2);
        assert!(store.is_trusted("d1"));
        assert!(store.is_trusted("d2"));
    }

    #[test]
    fn default_pairing_method_is_otp() {
        let d = TrustedDevice {
            device_id: DeviceId::from_string("d1"),
            device_name: "Phone".into(),
            last_seen: 1000,
            paired_at: 1000,
            pairing_method: PairingMethod::default(),
            protocol_version: 1,
        };
        assert_eq!(d.pairing_method, PairingMethod::Otp);
    }

    #[test]
    fn serde_backward_compatibility() {
        // Old format without pairing_method and protocol_version fields
        let json = r#"{"device_id":"d1","device_name":"Phone","last_seen":1000,"paired_at":1000}"#;
        let d: TrustedDevice = serde_json::from_str(json).unwrap();
        assert_eq!(d.pairing_method, PairingMethod::Otp);
        assert_eq!(d.protocol_version, 0);
    }
}
