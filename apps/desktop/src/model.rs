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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContextRuleId(String);

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
impl_id!(ContextRuleId);

// ── Domain types ─────────────────────────────────────────────

/// Persistence envelope — wraps all stored data with schema metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub schema: u32,
    pub created_with: String,
    pub last_saved_with: String,
    pub devices: Vec<TrustedDevice>,
    pub profiles: Vec<Profile>,
    #[serde(default)]
    pub context_rules: Vec<ContextRule>,
}

impl Document {
    pub fn empty() -> Self {
        Self {
            schema: 1,
            created_with: env!("CARGO_PKG_VERSION").into(),
            last_saved_with: env!("CARGO_PKG_VERSION").into(),
            devices: Vec::new(),
            profiles: vec![Profile::default()],
            context_rules: Vec::new(),
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

// ── Context domain types ─────────────────────────────────────

/// A process-to-Profile mapping.
/// The process_name is stored normalized (trimmed, lowercased).
/// All matching uses exact case-insensitive comparison.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextRule {
    pub id: ContextRuleId,
    pub process_name: String,
    pub profile_id: ProfileId,
}

/// Represents the observed foreground context. v0.3: process name only.
#[derive(Debug, Clone, PartialEq)]
pub struct ContextSnapshot {
    pub foreground_process: String,
}

/// Normalize a process name for canonical storage and matching.
/// Semantics: trim, case-insensitive exact match.
pub fn normalize_process_name(name: &str) -> String {
    name.trim().to_lowercase()
}

/// How the active runtime Profile is determined.
#[derive(Debug, Clone, PartialEq)]
pub enum SelectionMode {
    Automatic,
    Manual(ProfileId),
}

/// Runtime state of active Profile resolution. Not persisted.
#[derive(Debug, Clone, PartialEq)]
pub struct ProfileRuntime {
    pub active_profile_id: Option<ProfileId>,
    pub selection_mode: SelectionMode,
    pub latest_context: Option<ContextSnapshot>,
}

impl Default for ProfileRuntime {
    fn default() -> Self {
        Self {
            active_profile_id: None,
            selection_mode: SelectionMode::Automatic,
            latest_context: None,
        }
    }
}

impl ProfileRuntime {
    /// Reconcile runtime state after a Document mutation.
    /// Ensures ProfileRuntime is left in a fully consistent state:
    /// stale Manual → Automatic re-resolution, stale active_profile_id → re-resolve.
    pub fn reconcile(&mut self, doc: &Document) {
        let need_resolve = match &self.selection_mode {
            SelectionMode::Manual(pid) => {
                if !doc.profiles.iter().any(|p| p.id == *pid) {
                    self.selection_mode = SelectionMode::Automatic;
                    true
                } else {
                    false
                }
            }
            _ => {
                if let Some(ref pid) = self.active_profile_id {
                    !doc.profiles.iter().any(|p| p.id == *pid)
                } else {
                    false
                }
            }
        };
        if need_resolve {
            self.active_profile_id = resolve_active_profile(
                &doc.profiles,
                &doc.context_rules,
                self.latest_context.as_ref(),
                &self.selection_mode,
            );
        }
    }
}

/// Transport-neutral transition record for one observation cycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeTransition {
    pub context_changed: bool,
    pub previous_profile_id: Option<ProfileId>,
    pub active_profile_id: Option<ProfileId>,
}

impl RuntimeTransition {
    pub fn active_profile_changed(&self) -> bool {
        self.previous_profile_id != self.active_profile_id
    }
}

/// Pure function: resolve the active ProfileId given context, rules, and mode.
/// Does NOT mutate anything.
pub fn resolve_active_profile(
    profiles: &[Profile],
    rules: &[ContextRule],
    snapshot: Option<&ContextSnapshot>,
    mode: &SelectionMode,
) -> Option<ProfileId> {
    match mode {
        SelectionMode::Manual(pid) => {
            if profiles.iter().any(|p| p.id == *pid) {
                return Some(pid.clone());
            }
        }
        SelectionMode::Automatic => {}
    }
    // Automatic resolution (or manual stale fallback)
    if let Some(snap) = snapshot {
        let observed = normalize_process_name(&snap.foreground_process);
        if let Some(rule) = rules
            .iter()
            .find(|r| normalize_process_name(&r.process_name) == observed)
        {
            if profiles.iter().any(|p| p.id == rule.profile_id) {
                return Some(rule.profile_id.clone());
            }
        }
    }
    profiles.first().map(|p| p.id.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── normalize_process_name ──

    #[test]
    fn normalise_trims_whitespace() {
        assert_eq!(normalize_process_name("  code.exe  "), "code.exe");
    }

    #[test]
    fn normalise_lowercases() {
        assert_eq!(normalize_process_name("CODE.EXE"), "code.exe");
    }

    #[test]
    fn normalise_empty_string_returns_empty() {
        assert_eq!(normalize_process_name(""), "");
    }

    #[test]
    fn normalise_whitespace_only_returns_empty() {
        assert_eq!(normalize_process_name("   "), "");
    }

    // ── resolve_active_profile ──

    fn two_profiles() -> Vec<Profile> {
        vec![
            Profile {
                id: ProfileId::from_string("alpha"),
                name: "Alpha".into(),
                pages: vec![Page::default()],
            },
            Profile {
                id: ProfileId::from_string("beta"),
                name: "Beta".into(),
                pages: vec![Page::default()],
            },
        ]
    }

    #[test]
    fn resolve_rule_match_selects_matching_profile() {
        let profiles = two_profiles();
        let rules = vec![ContextRule {
            id: ContextRuleId::from_string("rule-1"),
            process_name: "code.exe".into(),
            profile_id: ProfileId::from_string("beta"),
        }];
        let snap = Some(ContextSnapshot {
            foreground_process: "CODE.exe".into(),
        });
        let result =
            resolve_active_profile(&profiles, &rules, snap.as_ref(), &SelectionMode::Automatic);
        assert_eq!(result, Some(ProfileId::from_string("beta")));
    }

    #[test]
    fn resolve_no_rule_match_returns_first_profile() {
        let profiles = two_profiles();
        let rules = vec![];
        let snap = Some(ContextSnapshot {
            foreground_process: "unknown.exe".into(),
        });
        let result =
            resolve_active_profile(&profiles, &rules, snap.as_ref(), &SelectionMode::Automatic);
        assert_eq!(result, Some(ProfileId::from_string("alpha")));
    }

    #[test]
    fn resolve_manual_valid_returns_manual_profile() {
        let profiles = two_profiles();
        let result = resolve_active_profile(
            &profiles,
            &[],
            None,
            &SelectionMode::Manual(ProfileId::from_string("beta")),
        );
        assert_eq!(result, Some(ProfileId::from_string("beta")));
    }

    #[test]
    fn resolve_manual_stale_falls_through_to_automatic() {
        let profiles = two_profiles();
        let rules = vec![];
        let snap = Some(ContextSnapshot {
            foreground_process: "anything.exe".into(),
        });
        let result = resolve_active_profile(
            &profiles,
            &rules,
            snap.as_ref(),
            &SelectionMode::Manual(ProfileId::from_string("nonexistent")),
        );
        assert_eq!(result, Some(ProfileId::from_string("alpha")));
    }

    #[test]
    fn resolve_no_snapshot_returns_first_profile() {
        let profiles = two_profiles();
        let result = resolve_active_profile(&profiles, &[], None, &SelectionMode::Automatic);
        assert_eq!(result, Some(ProfileId::from_string("alpha")));
    }

    #[test]
    fn resolve_empty_profiles_returns_none() {
        let result = resolve_active_profile(&[], &[], None, &SelectionMode::Automatic);
        assert_eq!(result, None);
    }

    #[test]
    fn resolve_rule_points_to_nonexistent_profile_falls_to_first() {
        let profiles = two_profiles();
        let rules = vec![ContextRule {
            id: ContextRuleId::from_string("rule-1"),
            process_name: "code.exe".into(),
            profile_id: ProfileId::from_string("ghost"),
        }];
        let snap = Some(ContextSnapshot {
            foreground_process: "code.exe".into(),
        });
        let result =
            resolve_active_profile(&profiles, &rules, snap.as_ref(), &SelectionMode::Automatic);
        assert_eq!(result, Some(ProfileId::from_string("alpha")));
    }

    #[test]
    fn resolve_non_normalized_context_rule_matches() {
        let profiles = two_profiles();
        let rules = vec![ContextRule {
            id: ContextRuleId::from_string("rule-1"),
            process_name: " Code.EXE ".into(),
            profile_id: ProfileId::from_string("beta"),
        }];
        let snap = Some(ContextSnapshot {
            foreground_process: "code.exe".into(),
        });
        let result =
            resolve_active_profile(&profiles, &rules, snap.as_ref(), &SelectionMode::Automatic);
        assert_eq!(result, Some(ProfileId::from_string("beta")));
    }

    #[test]
    fn resolve_substring_does_not_match() {
        let profiles = two_profiles();
        let rules = vec![ContextRule {
            id: ContextRuleId::from_string("rule-1"),
            process_name: "chrome.exe".into(),
            profile_id: ProfileId::from_string("beta"),
        }];
        let snap = Some(ContextSnapshot {
            foreground_process: "chrome".into(),
        });
        let result =
            resolve_active_profile(&profiles, &rules, snap.as_ref(), &SelectionMode::Automatic);
        // chrome != chrome.exe -> no match, falls to first profile
        assert_eq!(result, Some(ProfileId::from_string("alpha")));
    }

    // ── ProfileRuntime::reconcile ──

    fn doc_with_profiles(ids: &[&str]) -> Document {
        let mut doc = Document::empty();
        doc.profiles = ids
            .iter()
            .map(|id| Profile {
                id: ProfileId::from_string(*id),
                name: id.to_string(),
                pages: vec![Page::default()],
            })
            .collect();
        doc
    }

    #[test]
    fn reconcile_stale_manual_transitions_to_automatic() {
        let mut rt = ProfileRuntime {
            active_profile_id: Some(ProfileId::from_string("goner")),
            selection_mode: SelectionMode::Manual(ProfileId::from_string("goner")),
            latest_context: None,
        };
        let doc = doc_with_profiles(&["alpha", "beta"]);
        rt.reconcile(&doc);
        assert_eq!(rt.selection_mode, SelectionMode::Automatic);
        assert_eq!(rt.active_profile_id, Some(ProfileId::from_string("alpha")));
    }

    #[test]
    fn reconcile_valid_manual_unchanged() {
        let mut rt = ProfileRuntime {
            active_profile_id: Some(ProfileId::from_string("beta")),
            selection_mode: SelectionMode::Manual(ProfileId::from_string("beta")),
            latest_context: None,
        };
        let doc = doc_with_profiles(&["alpha", "beta"]);
        rt.reconcile(&doc);
        assert_eq!(
            rt.selection_mode,
            SelectionMode::Manual(ProfileId::from_string("beta"))
        );
        assert_eq!(rt.active_profile_id, Some(ProfileId::from_string("beta")));
    }

    #[test]
    fn reconcile_stale_manual_with_latest_context_resolves_to_rule_target() {
        let mut doc = doc_with_profiles(&["alpha", "beta"]);
        doc.context_rules.push(ContextRule {
            id: ContextRuleId::from_string("r1"),
            process_name: " code.exe ".into(),
            profile_id: ProfileId::from_string("beta"),
        });
        let mut rt = ProfileRuntime {
            active_profile_id: Some(ProfileId::from_string("goner")),
            selection_mode: SelectionMode::Manual(ProfileId::from_string("goner")),
            latest_context: Some(ContextSnapshot {
                foreground_process: "Code.EXE".into(),
            }),
        };
        rt.reconcile(&doc);
        assert_eq!(rt.selection_mode, SelectionMode::Automatic);
        assert_eq!(rt.active_profile_id, Some(ProfileId::from_string("beta")));
    }

    #[test]
    fn reconcile_stale_active_profile_id_re_resolves() {
        let mut doc = doc_with_profiles(&["alpha", "beta"]);
        doc.context_rules.push(ContextRule {
            id: ContextRuleId::from_string("r1"),
            process_name: "spotify.exe".into(),
            profile_id: ProfileId::from_string("beta"),
        });
        let mut rt = ProfileRuntime {
            active_profile_id: Some(ProfileId::from_string("goner")),
            selection_mode: SelectionMode::Automatic,
            latest_context: Some(ContextSnapshot {
                foreground_process: "Spotify.exe".into(),
            }),
        };
        rt.reconcile(&doc);
        // Rule match picks beta over alpha
        assert_eq!(rt.active_profile_id, Some(ProfileId::from_string("beta")));
    }

    #[test]
    fn reconcile_valid_active_profile_id_unchanged() {
        let mut rt = ProfileRuntime {
            active_profile_id: Some(ProfileId::from_string("alpha")),
            selection_mode: SelectionMode::Automatic,
            latest_context: None,
        };
        let doc = doc_with_profiles(&["alpha", "beta"]);
        rt.reconcile(&doc);
        assert_eq!(rt.active_profile_id, Some(ProfileId::from_string("alpha")));
    }

    #[test]
    fn reconcile_automatic_no_active_profile_id_stays_none() {
        let mut rt = ProfileRuntime {
            active_profile_id: None,
            selection_mode: SelectionMode::Automatic,
            latest_context: None,
        };
        let doc = doc_with_profiles(&["alpha", "beta"]);
        rt.reconcile(&doc);
        assert_eq!(rt.active_profile_id, None);
    }

    #[test]
    fn reconcile_stale_manual_no_profiles_resolves_to_none() {
        let doc = Document::empty();
        // Empty the profiles list
        let empty_doc = Document {
            profiles: vec![],
            ..doc
        };
        let mut rt = ProfileRuntime {
            active_profile_id: Some(ProfileId::from_string("goner")),
            selection_mode: SelectionMode::Manual(ProfileId::from_string("goner")),
            latest_context: None,
        };
        rt.reconcile(&empty_doc);
        assert_eq!(rt.selection_mode, SelectionMode::Automatic);
        assert_eq!(rt.active_profile_id, None);
    }

    // ── Document / Serialization ──

    #[test]
    fn document_empty_has_default_context_rules() {
        let doc = Document::empty();
        assert_eq!(doc.schema, 1);
        assert_eq!(doc.context_rules, vec![]);
        assert_eq!(doc.profiles.len(), 1);
    }

    #[test]
    fn serialization_round_trip_empty() {
        let doc = Document::empty();
        let json = serde_json::to_string(&doc).unwrap();
        let back: Document = serde_json::from_str(&json).unwrap();
        assert_eq!(doc, back);
    }

    #[test]
    fn serialization_round_trip_with_context_rules() {
        let mut doc = Document::empty();
        doc.context_rules = vec![ContextRule {
            id: ContextRuleId::from_string("r1"),
            process_name: "code.exe".into(),
            profile_id: ProfileId::from_string(doc.profiles[0].id.as_str()),
        }];
        let json = serde_json::to_string(&doc).unwrap();
        let back: Document = serde_json::from_str(&json).unwrap();
        assert_eq!(doc, back);
    }

    #[test]
    fn deserialize_v0_2_document_without_context_rules() {
        let v0_2_json = r#"{
            "schema": 1,
            "created_with": "0.2.0",
            "last_saved_with": "0.2.0",
            "devices": [],
            "profiles": [{"id": "p1", "name": "Default", "pages": [{"id": "pg1", "name": "Page 1", "buttons": []}]}]
        }"#;
        let doc: Document = serde_json::from_str(v0_2_json).unwrap();
        assert_eq!(doc.schema, 1);
        assert_eq!(doc.context_rules, vec![]);
        assert_eq!(doc.profiles.len(), 1);
    }

    #[test]
    fn deserialize_back_compat_context_rules_defaults_to_empty() {
        let v0_2_json = r#"{
            "schema": 1,
            "created_with": "0.2.0",
            "last_saved_with": "0.2.0",
            "devices": [],
            "profiles": []
        }"#;
        let doc: Document = serde_json::from_str(v0_2_json).unwrap();
        assert_eq!(doc.context_rules, Vec::<ContextRule>::new());
    }

    #[test]
    fn serialization_back_compat_profile_unchanged() {
        let v0_2_json = r#"{
            "schema": 1,
            "created_with": "0.2.0",
            "last_saved_with": "0.2.0",
            "devices": [],
            "profiles": [{"id": "p1", "name": "Legacy", "pages": [{"id": "pg1", "name": "Page 1", "buttons": []}]}]
        }"#;
        let doc: Document = serde_json::from_str(v0_2_json).unwrap();
        assert_eq!(doc.profiles[0].name, "Legacy");
        let back = serde_json::to_string(&doc).unwrap();
        assert!(back.contains(r#""context_rules":[]"#));
    }

    #[test]
    fn profile_runtime_default_state() {
        let rt = ProfileRuntime::default();
        assert_eq!(rt.active_profile_id, None);
        assert_eq!(rt.selection_mode, SelectionMode::Automatic);
        assert_eq!(rt.latest_context, None);
    }

    // ── RuntimeTransition ──

    #[test]
    fn transition_active_profile_unchanged() {
        let t = RuntimeTransition {
            context_changed: false,
            previous_profile_id: Some(ProfileId::from_string("a")),
            active_profile_id: Some(ProfileId::from_string("a")),
        };
        assert!(!t.active_profile_changed());
    }

    #[test]
    fn transition_active_profile_changed_a_to_b() {
        let t = RuntimeTransition {
            context_changed: true,
            previous_profile_id: Some(ProfileId::from_string("a")),
            active_profile_id: Some(ProfileId::from_string("b")),
        };
        assert!(t.active_profile_changed());
    }

    #[test]
    fn transition_active_profile_changed_none_to_a() {
        let t = RuntimeTransition {
            context_changed: true,
            previous_profile_id: None,
            active_profile_id: Some(ProfileId::from_string("a")),
        };
        assert!(t.active_profile_changed());
    }

    #[test]
    fn transition_active_profile_changed_a_to_none() {
        let t = RuntimeTransition {
            context_changed: true,
            previous_profile_id: Some(ProfileId::from_string("a")),
            active_profile_id: None,
        };
        assert!(t.active_profile_changed());
    }
}
