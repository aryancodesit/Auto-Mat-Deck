use std::sync::{Condvar, Mutex};

use crate::model::{Profile, ProfileId, RuntimeTransition};

/// Latest-value synchronization cell.
/// Bounded O(1) storage for at most one RuntimeTransition.
/// Observer overwrites without waiting; projection thread reads latest.
/// Notifications are advisory — the stored value is authoritative.
pub(crate) struct TransitionCell {
    pub(crate) latest: Mutex<Option<RuntimeTransition>>,
    notified: Condvar,
}

impl TransitionCell {
    pub fn new() -> Self {
        Self {
            latest: Mutex::new(None),
            notified: Condvar::new(),
        }
    }

    /// Store a transition, replacing any prior value.
    /// Never blocks (no I/O, no consumer dependency).
    /// Notifies one waiting consumer (advisory).
    pub fn store(&self, transition: RuntimeTransition) {
        *self.latest.lock().unwrap() = Some(transition);
        self.notified.notify_one();
    }

    /// Take the latest transition, leaving the cell empty.
    /// Returns None if no transition has been stored since last take.
    #[allow(dead_code)]
    pub fn take(&self) -> Option<RuntimeTransition> {
        self.latest.lock().unwrap().take()
    }

    /// Block until a transition is available, then take it.
    /// Returns immediately if one is already stored.
    #[allow(dead_code)]
    pub fn wait_and_take(&self) -> RuntimeTransition {
        let mut guard = self.latest.lock().unwrap();
        loop {
            if let Some(t) = guard.take() {
                return t;
            }
            guard = self.notified.wait(guard).unwrap();
        }
    }

    /// Wait up to `dur` for a transition, then take whatever is available.
    /// Returns `None` if no transition arrived within the timeout.
    pub fn wait_and_take_timeout(&self, dur: std::time::Duration) -> Option<RuntimeTransition> {
        let mut guard = self.latest.lock().unwrap();
        loop {
            if let Some(t) = guard.take() {
                return Some(t);
            }
            let (new_guard, result) = self.notified.wait_timeout(guard, dur).unwrap();
            guard = new_guard;
            if let Some(t) = guard.take() {
                return Some(t);
            }
            if result.timed_out() {
                return None;
            }
        }
    }
}

/// Transport-neutral derived state produced from a RuntimeTransition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeProjection {
    pub context_changed: bool,
    pub active_profile_changed: bool,
    pub previous_profile_id: Option<String>,
    pub active_profile_id: Option<String>,
}

/// Projection of the active Profile's control surface.
/// Preserves the certified Profile → Pages → Buttons hierarchy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlSurfaceState {
    pub profile_id: Option<String>,
    pub profile_name: Option<String>,
    pub pages: Option<Vec<PageProjection>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageProjection {
    pub page_id: String,
    pub name: String,
    pub buttons: Vec<ButtonProjection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ButtonProjection {
    pub button_id: String,
    pub label: String,
}

/// Outcome of attempting to derive a ControlSurfaceState.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DerivationResult {
    /// Valid projection state to publish.
    Published(ControlSurfaceState),
    /// Derivation failed (e.g. active Profile ID not found in Document).
    /// No fabricated projection; caller must retain current channel state.
    Failed,
}

/// Pure function: derive ControlSurfaceState from active Profile identity
/// and the authoritative Profile list. No I/O, no side effects.
pub fn derive_control_surface(
    active_profile_id: Option<&ProfileId>,
    profiles: &[Profile],
) -> DerivationResult {
    let Some(pid) = active_profile_id else {
        return DerivationResult::Published(ControlSurfaceState {
            profile_id: None,
            profile_name: None,
            pages: None,
        });
    };

    let profile = match profiles.iter().find(|p| p.id == *pid) {
        Some(p) => p,
        None => return DerivationResult::Failed,
    };

    let pages: Vec<PageProjection> = profile
        .pages
        .iter()
        .map(|page| PageProjection {
            page_id: page.id.as_str().to_string(),
            name: page.name.clone(),
            buttons: page
                .buttons
                .iter()
                .map(|btn| ButtonProjection {
                    button_id: btn.id.as_str().to_string(),
                    label: btn.label.clone(),
                })
                .collect(),
        })
        .collect();

    DerivationResult::Published(ControlSurfaceState {
        profile_id: Some(profile.id.as_str().to_string()),
        profile_name: Some(profile.name.clone()),
        pages: Some(pages),
    })
}

/// Pure function: RuntimeTransition → RuntimeProjection.
/// No I/O, no side effects, no state.
pub fn project(transition: &RuntimeTransition) -> RuntimeProjection {
    RuntimeProjection {
        context_changed: transition.context_changed,
        active_profile_changed: transition.active_profile_changed(),
        previous_profile_id: transition
            .previous_profile_id
            .as_ref()
            .map(|id| id.as_str().to_string()),
        active_profile_id: transition
            .active_profile_id
            .as_ref()
            .map(|id| id.as_str().to_string()),
    }
}

/// Decides whether a projection should be published.
/// Owns all suppression logic. Stateless across resets.
pub struct PublicationPolicy {
    last_emitted: Option<RuntimeProjection>,
}

impl PublicationPolicy {
    pub fn new() -> Self {
        Self { last_emitted: None }
    }

    /// Returns true if the projection should be published.
    /// First projection always publishes; subsequent projections
    /// publish only when they differ from the last emitted.
    pub fn should_publish(&mut self, projection: &RuntimeProjection) -> bool {
        match &self.last_emitted {
            None => {
                self.last_emitted = Some(projection.clone());
                true
            }
            Some(last) if last == projection => false,
            Some(_) => {
                self.last_emitted = Some(projection.clone());
                true
            }
        }
    }
}

/// Abstract interface for projection delivery.
/// Implementations must not modify, enrich, or derive projection data.
pub trait ProjectionPublisher: Send + Sync {
    fn publish(&self, projection: &RuntimeProjection);
}

/// Decides whether a ControlSurfaceState projection should be published.
/// Same suppression logic as PublicationPolicy but for ControlSurfaceState.
pub struct ControlSurfacePublicationPolicy {
    last_emitted: Option<ControlSurfaceState>,
}

impl ControlSurfacePublicationPolicy {
    pub fn new() -> Self {
        Self { last_emitted: None }
    }

    /// Returns true if the projection should be published.
    /// First projection always publishes; subsequent projections
    /// publish only when they differ from the last emitted.
    /// DerivationFailure always suppresses (channel unchanged).
    pub fn should_publish(&mut self, result: &DerivationResult) -> bool {
        match result {
            DerivationResult::Failed => false,
            DerivationResult::Published(state) => match &self.last_emitted {
                None => {
                    self.last_emitted = Some(state.clone());
                    true
                }
                Some(last) if last == state => false,
                Some(_) => {
                    self.last_emitted = Some(state.clone());
                    true
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ButtonId, PageId, ProfileId};
    use std::sync::Arc;

    fn t(context_changed: bool, prev: Option<&str>, active: Option<&str>) -> RuntimeTransition {
        RuntimeTransition {
            context_changed,
            previous_profile_id: prev.map(ProfileId::from_string),
            active_profile_id: active.map(ProfileId::from_string),
        }
    }

    // ── Architectural invariants ──

    #[test]
    fn deterministic_projection() {
        let tr = t(true, Some("a"), Some("b"));
        assert_eq!(project(&tr), project(&tr));
    }

    #[test]
    fn policy_suppresses_duplicate() {
        let mut policy = PublicationPolicy::new();
        let p = project(&t(true, Some("a"), Some("b")));
        assert!(policy.should_publish(&p));
        assert!(!policy.should_publish(&p));
    }

    #[test]
    fn policy_allows_change_after_duplicate() {
        let mut policy = PublicationPolicy::new();
        let p1 = project(&t(true, Some("a"), Some("b")));
        let p2 = project(&t(true, Some("a"), Some("c")));
        assert!(policy.should_publish(&p1));
        assert!(policy.should_publish(&p2));
    }

    // ── ProjectionEngine field mapping ──

    #[test]
    fn context_changed_true_mapped() {
        let p = project(&t(true, None, None));
        assert!(p.context_changed);
    }

    #[test]
    fn context_changed_false_mapped() {
        let p = project(&t(false, None, None));
        assert!(!p.context_changed);
    }

    #[test]
    fn active_profile_changed_true_mapped() {
        let p = project(&t(true, Some("a"), Some("b")));
        assert!(p.active_profile_changed);
    }

    #[test]
    fn active_profile_changed_false_mapped() {
        let p = project(&t(true, Some("a"), Some("a")));
        assert!(!p.active_profile_changed);
    }

    #[test]
    fn previous_profile_mapped() {
        let p = project(&t(true, Some("alpha"), Some("beta")));
        assert_eq!(p.previous_profile_id, Some("alpha".into()));
    }

    #[test]
    fn active_profile_mapped() {
        let p = project(&t(true, Some("alpha"), Some("beta")));
        assert_eq!(p.active_profile_id, Some("beta".into()));
    }

    #[test]
    fn none_profiles_handled() {
        let p = project(&t(true, None, None));
        assert_eq!(p.previous_profile_id, None);
        assert_eq!(p.active_profile_id, None);
    }

    // ── PublicationPolicy ──

    #[test]
    fn policy_first_always_publishes() {
        let mut policy = PublicationPolicy::new();
        assert!(policy.should_publish(&project(&t(true, None, None))));
    }

    #[test]
    fn policy_reset_allows_duplicate() {
        let p = project(&t(true, None, None));
        let mut policy = PublicationPolicy::new();
        assert!(policy.should_publish(&p));
        let mut policy2 = PublicationPolicy::new();
        assert!(policy2.should_publish(&p));
    }

    #[test]
    fn policy_does_not_run_engine() {
        let mut policy = PublicationPolicy::new();
        let p1 = project(&t(true, Some("a"), Some("b")));
        let p2 = project(&t(true, Some("a"), Some("b")));
        assert!(policy.should_publish(&p1));
        assert!(!policy.should_publish(&p2));
    }

    // ── TransitionCell (latest-value sync) ──

    #[test]
    fn cell_store_then_take_returns_same_value() {
        let cell = TransitionCell::new();
        let tr = t(true, Some("a"), Some("b"));
        cell.store(tr.clone());
        assert_eq!(cell.take(), Some(tr));
    }

    #[test]
    fn cell_overwrite_drops_prior() {
        let cell = TransitionCell::new();
        cell.store(t(true, Some("a"), Some("b")));
        cell.store(t(true, Some("c"), Some("d")));
        assert_eq!(
            cell.take().map(|r| r.active_profile_id),
            Some(Some(ProfileId::from_string("d")))
        );
    }

    #[test]
    fn cell_read_on_empty_returns_none() {
        let cell = TransitionCell::new();
        assert_eq!(cell.take(), None);
    }

    #[test]
    fn cell_multiple_writes_single_read() {
        let cell = TransitionCell::new();
        for i in 0..10 {
            cell.store(t(true, Some("a"), Some(&format!("p{i}"))));
        }
        let taken = cell.take();
        assert_eq!(
            taken.as_ref().and_then(|r| r
                .active_profile_id
                .as_ref()
                .map(|id| id.as_str().to_string())),
            Some("p9".into())
        );
    }

    // ── Integration: mock publisher ──

    struct MockPublisher {
        calls: std::sync::Mutex<Vec<RuntimeProjection>>,
    }

    impl MockPublisher {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                calls: std::sync::Mutex::new(vec![]),
            })
        }

        fn count(&self) -> usize {
            self.calls.lock().unwrap().len()
        }
    }

    impl ProjectionPublisher for MockPublisher {
        fn publish(&self, projection: &RuntimeProjection) {
            self.calls.lock().unwrap().push(projection.clone());
        }
    }

    #[test]
    fn integration_unique_projection_publishes_once() {
        let pub_ = MockPublisher::new();
        let mut policy = PublicationPolicy::new();
        let tr = t(true, Some("a"), Some("b"));
        let proj = project(&tr);
        if policy.should_publish(&proj) {
            pub_.publish(&proj);
        }
        assert_eq!(pub_.count(), 1);
    }

    #[test]
    fn integration_dedup_suppresses_publish() {
        let pub_ = MockPublisher::new();
        let mut policy = PublicationPolicy::new();
        let tr = t(true, Some("a"), Some("b"));
        let proj = project(&tr);
        // First call publishes
        if policy.should_publish(&proj) {
            pub_.publish(&proj);
        }
        // Second identical call suppresses
        if policy.should_publish(&proj) {
            pub_.publish(&proj);
        }
        assert_eq!(pub_.count(), 1);
    }

    #[test]
    fn integration_publisher_failure_is_isolated() {
        let pub_ = MockPublisher::new();
        let mut policy = PublicationPolicy::new();
        // First publication "fails" — we simply don't call it
        let tr1 = t(true, Some("a"), Some("b"));
        let p1 = project(&tr1);
        if policy.should_publish(&p1) {
            // simulate failure: skip publish
        }
        // Second publication (after a change) succeeds
        let tr2 = t(true, Some("a"), Some("c"));
        let p2 = project(&tr2);
        if policy.should_publish(&p2) {
            pub_.publish(&p2);
        }
        assert_eq!(pub_.count(), 1);
    }

    // ── Compile-time guards ──

    #[test]
    fn policy_receives_immutable_projection() {
        let p = project(&t(true, None, None));
        let mut policy = PublicationPolicy::new();
        // Policy receives &RuntimeProjection — cannot mutate
        let _ = policy.should_publish(&p);
    }

    // ── Control Surface Derivation ──

    fn profile(name: &str, pages: Vec<PageProjection>) -> Profile {
        Profile {
            id: ProfileId::from_string(name),
            name: name.to_string(),
            pages: pages
                .into_iter()
                .map(|pp| crate::model::Page {
                    id: PageId::from_string(&pp.page_id),
                    name: pp.name,
                    buttons: pp
                        .buttons
                        .into_iter()
                        .map(|bp| crate::model::Button {
                            id: ButtonId::from_string(&bp.button_id),
                            label: bp.label,
                            action: crate::model::ActionReference {
                                action_name: String::new(),
                                payload: serde_json::Value::Null,
                            },
                        })
                        .collect(),
                })
                .collect(),
        }
    }

    #[test]
    fn derive_no_active_profile_returns_null_triple() {
        let profiles = vec![profile("p1", vec![])];
        let result = derive_control_surface(None, &profiles);
        match result {
            DerivationResult::Published(state) => {
                assert_eq!(state.profile_id, None);
                assert_eq!(state.profile_name, None);
                assert_eq!(state.pages, None);
            }
            DerivationResult::Failed => panic!("expected Published(null triple)"),
        }
    }

    #[test]
    fn derive_active_profile_preserves_association() {
        let profiles = vec![profile(
            "coding",
            vec![PageProjection {
                page_id: "pg1".into(),
                name: "Main".into(),
                buttons: vec![],
            }],
        )];
        let pid = ProfileId::from_string("coding");
        let result = derive_control_surface(Some(&pid), &profiles);
        match result {
            DerivationResult::Published(state) => {
                assert_eq!(state.profile_id, Some("coding".into()));
                assert_eq!(state.profile_name, Some("coding".into()));
            }
            DerivationResult::Failed => panic!("expected Published"),
        }
    }

    #[test]
    fn derive_active_profile_name_preserved() {
        let mut p = profile("p1", vec![]);
        p.name = "Coding".into();
        let profiles = vec![p];
        let pid = ProfileId::from_string("p1");
        let result = derive_control_surface(Some(&pid), &profiles);
        match result {
            DerivationResult::Published(state) => {
                assert_eq!(state.profile_name, Some("Coding".into()));
            }
            DerivationResult::Failed => panic!("expected Published"),
        }
    }

    #[test]
    fn derive_page_order_preserved() {
        let profiles = vec![profile(
            "p1",
            vec![
                PageProjection {
                    page_id: "a".into(),
                    name: "A".into(),
                    buttons: vec![],
                },
                PageProjection {
                    page_id: "b".into(),
                    name: "B".into(),
                    buttons: vec![],
                },
                PageProjection {
                    page_id: "c".into(),
                    name: "C".into(),
                    buttons: vec![],
                },
            ],
        )];
        let pid = ProfileId::from_string("p1");
        let result = derive_control_surface(Some(&pid), &profiles);
        match result {
            DerivationResult::Published(state) => {
                let pages = state.pages.unwrap();
                assert_eq!(pages.len(), 3);
                assert_eq!(pages[0].page_id, "a");
                assert_eq!(pages[1].page_id, "b");
                assert_eq!(pages[2].page_id, "c");
            }
            DerivationResult::Failed => panic!("expected Published"),
        }
    }

    #[test]
    fn derive_page_ids_preserved() {
        let profiles = vec![profile(
            "p1",
            vec![PageProjection {
                page_id: "my-page-id".into(),
                name: "Main".into(),
                buttons: vec![],
            }],
        )];
        let pid = ProfileId::from_string("p1");
        let result = derive_control_surface(Some(&pid), &profiles);
        match result {
            DerivationResult::Published(state) => {
                assert_eq!(state.pages.unwrap()[0].page_id, "my-page-id");
            }
            DerivationResult::Failed => panic!("expected Published"),
        }
    }

    #[test]
    fn derive_page_names_preserved() {
        let profiles = vec![profile(
            "p1",
            vec![PageProjection {
                page_id: "pg1".into(),
                name: "My Page".into(),
                buttons: vec![],
            }],
        )];
        let pid = ProfileId::from_string("p1");
        let result = derive_control_surface(Some(&pid), &profiles);
        match result {
            DerivationResult::Published(state) => {
                assert_eq!(state.pages.unwrap()[0].name, "My Page");
            }
            DerivationResult::Failed => panic!("expected Published"),
        }
    }

    #[test]
    fn derive_button_order_preserved_per_page() {
        let profiles = vec![profile(
            "p1",
            vec![PageProjection {
                page_id: "pg1".into(),
                name: "Main".into(),
                buttons: vec![
                    ButtonProjection {
                        button_id: "btn1".into(),
                        label: "First".into(),
                    },
                    ButtonProjection {
                        button_id: "btn2".into(),
                        label: "Second".into(),
                    },
                ],
            }],
        )];
        let pid = ProfileId::from_string("p1");
        let result = derive_control_surface(Some(&pid), &profiles);
        match result {
            DerivationResult::Published(state) => {
                let buttons = &state.pages.unwrap()[0].buttons;
                assert_eq!(buttons.len(), 2);
                assert_eq!(buttons[0].button_id, "btn1");
                assert_eq!(buttons[1].button_id, "btn2");
            }
            DerivationResult::Failed => panic!("expected Published"),
        }
    }

    #[test]
    fn derive_button_ids_preserved() {
        let profiles = vec![profile(
            "p1",
            vec![PageProjection {
                page_id: "pg1".into(),
                name: "Main".into(),
                buttons: vec![ButtonProjection {
                    button_id: "my-btn-id".into(),
                    label: "Test".into(),
                }],
            }],
        )];
        let pid = ProfileId::from_string("p1");
        let result = derive_control_surface(Some(&pid), &profiles);
        match result {
            DerivationResult::Published(state) => {
                assert_eq!(state.pages.unwrap()[0].buttons[0].button_id, "my-btn-id");
            }
            DerivationResult::Failed => panic!("expected Published"),
        }
    }

    #[test]
    fn derive_button_labels_preserved() {
        let profiles = vec![profile(
            "p1",
            vec![PageProjection {
                page_id: "pg1".into(),
                name: "Main".into(),
                buttons: vec![ButtonProjection {
                    button_id: "b1".into(),
                    label: "Compile".into(),
                }],
            }],
        )];
        let pid = ProfileId::from_string("p1");
        let result = derive_control_surface(Some(&pid), &profiles);
        match result {
            DerivationResult::Published(state) => {
                assert_eq!(state.pages.unwrap()[0].buttons[0].label, "Compile");
            }
            DerivationResult::Failed => panic!("expected Published"),
        }
    }

    #[test]
    fn derive_active_profile_zero_pages_returns_empty_array() {
        let profiles = vec![profile("empty", vec![])];
        let pid = ProfileId::from_string("empty");
        let result = derive_control_surface(Some(&pid), &profiles);
        match result {
            DerivationResult::Published(state) => {
                assert_eq!(state.pages, Some(vec![]));
            }
            DerivationResult::Failed => panic!("expected Published"),
        }
    }

    #[test]
    fn derive_unresolved_profile_returns_failed() {
        let profiles = vec![profile("p1", vec![])];
        let pid = ProfileId::from_string("nonexistent");
        let result = derive_control_surface(Some(&pid), &profiles);
        assert_eq!(result, DerivationResult::Failed);
    }

    #[test]
    fn derive_action_reference_excluded() {
        let mut p = profile(
            "p1",
            vec![PageProjection {
                page_id: "pg1".into(),
                name: "Main".into(),
                buttons: vec![ButtonProjection {
                    button_id: "b1".into(),
                    label: "Test".into(),
                }],
            }],
        );
        // Set an action on the button
        p.pages[0].buttons[0].action = crate::model::ActionReference {
            action_name: "launch".into(),
            payload: serde_json::Value::String("notepad.exe".into()),
        };
        let profiles = vec![p];
        let pid = ProfileId::from_string("p1");
        let result = derive_control_surface(Some(&pid), &profiles);
        match result {
            DerivationResult::Published(state) => {
                let btn = &state.pages.unwrap()[0].buttons[0];
                assert_eq!(btn.button_id, "b1");
                assert_eq!(btn.label, "Test");
                // Only button_id and label exist — no action fields
            }
            DerivationResult::Failed => panic!("expected Published"),
        }
    }

    // ── ControlSurfacePublicationPolicy ──

    #[test]
    fn css_policy_first_always_publishes() {
        let mut policy = ControlSurfacePublicationPolicy::new();
        let result = DerivationResult::Published(ControlSurfaceState {
            profile_id: None,
            profile_name: None,
            pages: None,
        });
        assert!(policy.should_publish(&result));
    }

    #[test]
    fn css_policy_suppresses_duplicate() {
        let mut policy = ControlSurfacePublicationPolicy::new();
        let result = DerivationResult::Published(ControlSurfaceState {
            profile_id: Some("p1".into()),
            profile_name: Some("P1".into()),
            pages: Some(vec![]),
        });
        assert!(policy.should_publish(&result));
        assert!(!policy.should_publish(&result));
    }

    #[test]
    fn css_policy_allows_change_after_duplicate() {
        let mut policy = ControlSurfacePublicationPolicy::new();
        let r1 = DerivationResult::Published(ControlSurfaceState {
            profile_id: Some("p1".into()),
            profile_name: Some("P1".into()),
            pages: Some(vec![]),
        });
        let r2 = DerivationResult::Published(ControlSurfaceState {
            profile_id: Some("p2".into()),
            profile_name: Some("P2".into()),
            pages: Some(vec![]),
        });
        assert!(policy.should_publish(&r1));
        assert!(policy.should_publish(&r2));
    }

    #[test]
    fn css_policy_failure_suppresses() {
        let mut policy = ControlSurfacePublicationPolicy::new();
        assert!(!policy.should_publish(&DerivationResult::Failed));
    }

    #[test]
    fn css_policy_reset_allows_duplicate() {
        let result = DerivationResult::Published(ControlSurfaceState {
            profile_id: Some("p1".into()),
            profile_name: Some("P1".into()),
            pages: Some(vec![]),
        });
        let mut policy = ControlSurfacePublicationPolicy::new();
        assert!(policy.should_publish(&result));
        let mut policy2 = ControlSurfacePublicationPolicy::new();
        assert!(policy2.should_publish(&result));
    }
}
