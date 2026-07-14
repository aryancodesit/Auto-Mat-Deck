use std::sync::{Arc, Condvar, Mutex};

use crate::model::RuntimeTransition;

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

/// Temporary publisher that logs every received projection.
/// Used as a bootstrap implementation until transport publishers exist.
pub(crate) struct LoggingPublisher;

impl ProjectionPublisher for LoggingPublisher {
    fn publish(&self, projection: &RuntimeProjection) {
        log::info!(
            "Projection: ctx={} profile_changed={} prev={:?} active={:?}",
            projection.context_changed,
            projection.active_profile_changed,
            projection.previous_profile_id,
            projection.active_profile_id,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ProfileId;

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
}
