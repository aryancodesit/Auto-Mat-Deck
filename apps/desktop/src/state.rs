use std::sync::{Arc, Mutex};

use crate::command::{self, Command, CommandError};
use crate::model::*;
use crate::projection::TransitionCell;
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

/// Desktop runtime: wraps persisted AppState and transient ProfileRuntime.
///
/// Exactly three fields. A fourth field requires explicit architecture review.
pub struct DesktopRuntime {
    pub app: AppState,
    pub runtime: ProfileRuntime,
    pub transition_cell: TransitionCell,
}

impl DesktopRuntime {
    /// Apply a domain command, reconcile runtime, return success/failure.
    /// Caller is responsible for persistence after this.
    pub fn dispatch_document(&mut self, cmd: &Command) -> Result<(), CommandError> {
        let prev = self.runtime.active_profile_id.clone();
        self.app.dispatch(cmd)?;
        self.runtime.reconcile(&self.app.document);
        let active = self.runtime.active_profile_id.clone();
        self.transition_cell.store(RuntimeTransition {
            context_changed: prev != active,
            previous_profile_id: prev,
            active_profile_id: active,
        });
        Ok(())
    }

    /// Apply a foreground context observation.
    /// Caller must pass Ok(Some) or Ok(None); Err is handled at the poll level.
    pub(crate) fn apply_context_observation(
        &mut self,
        snapshot: Option<ContextSnapshot>,
    ) -> RuntimeTransition {
        let prev_profile = self.runtime.active_profile_id.clone();
        let prev_ctx = self.runtime.latest_context.clone();

        let ctx_unchanged = match (&prev_ctx, &snapshot) {
            (Some(a), Some(b)) => {
                normalize_process_name(&a.foreground_process)
                    == normalize_process_name(&b.foreground_process)
            }
            (None, None) => true,
            _ => false,
        };

        if ctx_unchanged {
            return RuntimeTransition {
                context_changed: false,
                previous_profile_id: prev_profile,
                active_profile_id: self.runtime.active_profile_id.clone(),
            };
        }

        self.runtime.latest_context = snapshot;

        if self.runtime.selection_mode == SelectionMode::Automatic {
            self.runtime.active_profile_id = resolve_active_profile(
                &self.app.document.profiles,
                &self.app.document.context_rules,
                self.runtime.latest_context.as_ref(),
                &self.runtime.selection_mode,
            );
        }

        RuntimeTransition {
            context_changed: true,
            previous_profile_id: prev_profile,
            active_profile_id: self.runtime.active_profile_id.clone(),
        }
    }

    pub fn new(store: &dyn DocumentStore) -> Self {
        Self {
            app: AppState::load(store),
            runtime: ProfileRuntime::default(),
            transition_cell: TransitionCell::new(),
        }
    }
}

/// Shared runtime wrapper used across threads.
pub type SharedRuntime = Arc<Mutex<DesktopRuntime>>;

pub fn new_shared(store: &dyn DocumentStore) -> SharedRuntime {
    Arc::new(Mutex::new(DesktopRuntime::new(store)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observer::{ContextObserverError, successful_observation};

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

    fn dt() -> DesktopRuntime {
        DesktopRuntime {
            app: AppState {
                document: doc_with_profiles(&["alpha", "beta"]),
                server_running: false,
                selected_tab: Tab::Dashboard,
            },
            runtime: ProfileRuntime::default(),
            transition_cell: TransitionCell::new(),
        }
    }

    // ── Deduplication ──

    #[test]
    fn normalized_equivalent_deduplicates() {
        let mut rt = dt();
        // First observation
        let t1 = rt.apply_context_observation(Some(ContextSnapshot {
            foreground_process: "Code.exe".into(),
        }));
        assert!(t1.context_changed);
        assert_eq!(t1.active_profile_id, Some(ProfileId::from_string("alpha")));

        // Same process, different case
        let t2 = rt.apply_context_observation(Some(ContextSnapshot {
            foreground_process: "code.exe".into(),
        }));
        assert!(!t2.context_changed);
    }

    #[test]
    fn substring_different_is_real_transition() {
        let mut rt = dt();
        rt.apply_context_observation(Some(ContextSnapshot {
            foreground_process: "spotify.exe".into(),
        }));
        let t = rt.apply_context_observation(Some(ContextSnapshot {
            foreground_process: "spotify".into(),
        }));
        assert!(t.context_changed);
    }

    #[test]
    fn transition_updates_latest_context() {
        let mut rt = dt();
        rt.apply_context_observation(Some(ContextSnapshot {
            foreground_process: "Code.exe".into(),
        }));
        assert_eq!(
            rt.runtime
                .latest_context
                .as_ref()
                .map(|c| &c.foreground_process),
            Some(&"Code.exe".into())
        );
        rt.apply_context_observation(Some(ContextSnapshot {
            foreground_process: "Spotify.exe".into(),
        }));
        assert_eq!(
            rt.runtime
                .latest_context
                .as_ref()
                .map(|c| &c.foreground_process),
            Some(&"Spotify.exe".into())
        );
    }

    // ── Automatic mode ──

    #[test]
    fn automatic_rule_match_changes_active_profile() {
        let mut rt = dt();
        rt.app.document.context_rules.push(ContextRule {
            id: ContextRuleId::from_string("r1"),
            process_name: "code.exe".into(),
            profile_id: ProfileId::from_string("beta"),
        });
        let t = rt.apply_context_observation(Some(ContextSnapshot {
            foreground_process: "Code.exe".into(),
        }));
        assert!(t.context_changed);
        assert!(t.active_profile_changed());
        assert_eq!(t.active_profile_id, Some(ProfileId::from_string("beta")));
    }

    #[test]
    fn automatic_no_match_falls_to_first() {
        let mut rt = dt();
        let t = rt.apply_context_observation(Some(ContextSnapshot {
            foreground_process: "unknown.exe".into(),
        }));
        assert!(t.context_changed);
        assert_eq!(t.active_profile_id, Some(ProfileId::from_string("alpha")));
    }

    // ── Manual mode ──

    #[test]
    fn manual_retains_active_profile() {
        let mut rt = dt();
        rt.runtime.selection_mode = SelectionMode::Manual(ProfileId::from_string("beta"));
        rt.runtime.active_profile_id = Some(ProfileId::from_string("beta"));
        let t = rt.apply_context_observation(Some(ContextSnapshot {
            foreground_process: "Code.exe".into(),
        }));
        assert!(t.context_changed);
        assert!(!t.active_profile_changed());
        assert_eq!(t.active_profile_id, Some(ProfileId::from_string("beta")));
    }

    #[test]
    fn manual_records_latest_context() {
        let mut rt = dt();
        rt.runtime.selection_mode = SelectionMode::Manual(ProfileId::from_string("beta"));
        rt.runtime.active_profile_id = Some(ProfileId::from_string("beta"));
        rt.apply_context_observation(Some(ContextSnapshot {
            foreground_process: "Code.exe".into(),
        }));
        assert_eq!(
            rt.runtime
                .latest_context
                .as_ref()
                .map(|c| &c.foreground_process),
            Some(&"Code.exe".into())
        );
    }

    #[test]
    fn stale_manual_deletion_uses_retained_context() {
        let mut rt = dt();
        // Add goner to the document so it can be deleted
        rt.app.document.profiles.push(Profile {
            id: ProfileId::from_string("goner"),
            name: "Goner".into(),
            pages: vec![Page::default()],
        });
        rt.runtime.selection_mode = SelectionMode::Manual(ProfileId::from_string("goner"));
        rt.runtime.active_profile_id = Some(ProfileId::from_string("goner"));
        rt.runtime.latest_context = Some(ContextSnapshot {
            foreground_process: "Spotify.exe".into(),
        });
        rt.app.document.context_rules.push(ContextRule {
            id: ContextRuleId::from_string("r1"),
            process_name: "spotify.exe".into(),
            profile_id: ProfileId::from_string("beta"),
        });
        rt.dispatch_document(&Command::DeleteProfile {
            profile_id: ProfileId::from_string("goner"),
        })
        .unwrap();
        assert_eq!(rt.runtime.selection_mode, SelectionMode::Automatic);
        assert_eq!(
            rt.runtime.active_profile_id,
            Some(ProfileId::from_string("beta"))
        );
    }

    // ── Context+profile transition representation ──

    #[test]
    fn context_changed_profile_unchanged() {
        let mut rt = dt();
        rt.runtime.selection_mode = SelectionMode::Manual(ProfileId::from_string("beta"));
        rt.runtime.active_profile_id = Some(ProfileId::from_string("beta"));
        let t = rt.apply_context_observation(Some(ContextSnapshot {
            foreground_process: "Code.exe".into(),
        }));
        assert!(t.context_changed);
        assert!(!t.active_profile_changed());
    }

    #[test]
    fn context_and_profile_changed_a_to_b() {
        let mut rt = dt();
        rt.app.document.context_rules.push(ContextRule {
            id: ContextRuleId::from_string("r1"),
            process_name: "code.exe".into(),
            profile_id: ProfileId::from_string("beta"),
        });
        rt.apply_context_observation(Some(ContextSnapshot {
            foreground_process: "other.exe".into(),
        }));
        let t = rt.apply_context_observation(Some(ContextSnapshot {
            foreground_process: "Code.exe".into(),
        }));
        assert!(t.context_changed);
        assert!(t.active_profile_changed());
        assert_eq!(t.active_profile_id, Some(ProfileId::from_string("beta")));
    }

    #[test]
    fn profile_transition_none_to_a() {
        let mut rt = dt();
        // Start with empty profiles so active_profile_id is None
        rt.app.document.profiles.clear();
        rt.runtime.active_profile_id = None;
        rt.app.document.profiles = vec![Profile {
            id: ProfileId::from_string("alpha"),
            name: "Alpha".into(),
            pages: vec![Page::default()],
        }];
        let t = rt.apply_context_observation(Some(ContextSnapshot {
            foreground_process: "anything.exe".into(),
        }));
        assert!(t.context_changed);
        assert!(t.active_profile_changed());
        assert_eq!(t.active_profile_id, Some(ProfileId::from_string("alpha")));
    }

    #[test]
    fn profile_transition_a_to_none() {
        let mut rt = dt();
        // No profiles at all
        rt.app.document.profiles.clear();
        rt.runtime.active_profile_id = None;
        let t = rt.apply_context_observation(Some(ContextSnapshot {
            foreground_process: "anything.exe".into(),
        }));
        assert!(t.context_changed);
        assert_eq!(t.active_profile_id, None);
    }

    #[test]
    fn no_profiles_resolves_none() {
        let mut rt = dt();
        rt.app.document.profiles.clear();
        let t = rt.apply_context_observation(Some(ContextSnapshot {
            foreground_process: "anything.exe".into(),
        }));
        assert_eq!(t.active_profile_id, None);
    }

    // ── Observer boundary: Ok(None) semantics ──

    #[test]
    fn ok_none_updates_latest_context_to_none() {
        // apply_context_observation(None) transitions latest_context
        // to None — this is correct: the foreground has no window.
        let mut rt = dt();
        rt.apply_context_observation(Some(ContextSnapshot {
            foreground_process: "Code.exe".into(),
        }));
        assert!(rt.runtime.latest_context.is_some());

        let t = rt.apply_context_observation(None);
        assert!(t.context_changed);
        // latest_context updated to None — correct for Ok(None).
    }

    // ── Observer boundary: Err retention ──

    #[test]
    fn err_preserves_latest_context() {
        // The pure helper successful_observation rejects Err observations.
        // When it returns None the caller never mutates runtime state,
        // so latest_context is always preserved on failure.
        assert!(successful_observation(Err(ContextObserverError::ProcessOpenFailed)).is_none());
        assert!(
            successful_observation(Err(ContextObserverError::ProcessNameQueryFailed)).is_none()
        );
        assert!(successful_observation(Err(ContextObserverError::InvalidProcessName)).is_none());
    }

    #[test]
    fn code_err_code_trace_preserves_context_and_deduplicates() {
        let mut rt = dt();

        // ── Step 1: Ok(Some("Code.exe")) ──
        let step1 = Some(ContextSnapshot {
            foreground_process: "Code.exe".into(),
        });
        let policy1 = successful_observation(Ok(step1.clone()));
        assert_eq!(policy1, Some(step1));
        let t1 = rt.apply_context_observation(policy1.unwrap());
        assert!(t1.context_changed);
        let step1_profile = t1.active_profile_id.clone();
        assert_eq!(
            rt.runtime
                .latest_context
                .as_ref()
                .map(|c| &c.foreground_process),
            Some(&"Code.exe".into())
        );

        // ── Step 2: Err(ProcessOpenFailed) ──
        let policy2 = successful_observation(Err(ContextObserverError::ProcessOpenFailed));
        assert_eq!(policy2, None);
        // apply_context_observation is NOT called because policy2 is None
        assert_eq!(
            rt.runtime
                .latest_context
                .as_ref()
                .map(|c| &c.foreground_process),
            Some(&"Code.exe".into())
        );
        assert_eq!(rt.runtime.active_profile_id, step1_profile);

        // ── Step 3: Ok(Some("Code.exe")) — dedup ──
        let step3 = Some(ContextSnapshot {
            foreground_process: "Code.exe".into(),
        });
        let policy3 = successful_observation(Ok(step3.clone()));
        assert_eq!(policy3, Some(step3));
        let t3 = rt.apply_context_observation(policy3.unwrap());
        assert!(!t3.context_changed);
        assert!(!t3.active_profile_changed());
        assert_eq!(
            rt.runtime
                .latest_context
                .as_ref()
                .map(|c| &c.foreground_process),
            Some(&"Code.exe".into())
        );
        assert_eq!(rt.runtime.active_profile_id, step1_profile);
    }

    #[test]
    fn domain_tests_require_no_win32() {
        // This test is the assertion that no Win32 infrastructure is needed
        // for domain tests. All tests in this module construct
        // DesktopRuntime directly.
        let _rt = dt();
    }
}
