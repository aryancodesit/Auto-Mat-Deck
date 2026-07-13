use serde::{Deserialize, Serialize};

// ── ID generation ────────────────────────────────────────────

fn new_id() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    format!("{:016x}{:016x}", now.as_secs(), now.subsec_nanos())
}

// ── ID types ─────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProfileId(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PageId(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ButtonId(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeviceId(String);

macro_rules! impl_id {
    ($name:ident) => {
        impl $name {
            pub fn new() -> Self {
                Self(new_id())
            }
            pub fn from_string(s: impl Into<String>) -> Self {
                Self(s.into())
            }
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

impl_id!(ProfileId);
impl_id!(PageId);
impl_id!(ButtonId);
impl_id!(DeviceId);

// ── Domain types ─────────────────────────────────────────────

/// Persistence envelope — wraps all stored data with schema metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub schema: u32,
    pub created_with: String,
    pub last_saved_with: String,
    pub devices: Vec<TrustedDevice>,
    pub profiles: Vec<Profile>,
}

impl Document {
    pub fn empty() -> Self {
        Self {
            schema: 1,
            created_with: env!("CARGO_PKG_VERSION").into(),
            last_saved_with: env!("CARGO_PKG_VERSION").into(),
            devices: Vec::new(),
            profiles: vec![Profile::default()],
        }
    }
}

/// A trusted remote device that has completed pairing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrustedDevice {
    pub device_id: DeviceId,
    pub device_name: String,
    pub last_seen: u64,
    pub paired_at: u64,
}

/// A named profile containing pages of buttons.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    pub id: ProfileId,
    pub name: String,
    pub pages: Vec<Page>,
}

impl Profile {
    pub fn default() -> Self {
        Self {
            id: ProfileId::new(),
            name: "Default".into(),
            pages: vec![Page::default()],
        }
    }
}

/// A page within a profile, containing buttons.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Page {
    pub id: PageId,
    pub name: String,
    pub buttons: Vec<Button>,
}

impl Page {
    pub fn default() -> Self {
        Self {
            id: PageId::new(),
            name: "Page 1".into(),
            buttons: Vec::new(),
        }
    }
}

/// A single actionable button on a page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Button {
    pub id: ButtonId,
    pub label: String,
    pub action: ActionReference,
}

/// Reference to an action registered in the action registry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionReference {
    pub action_name: String,
    #[serde(default)]
    pub payload: serde_json::Value,
}
