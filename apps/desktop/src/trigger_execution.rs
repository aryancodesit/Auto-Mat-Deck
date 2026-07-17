use crate::model::{Trigger, TriggerType, WorkflowId};

/// Result of evaluating a single trigger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TriggerEvaluationResult {
    pub trigger_id: String,
    pub trigger_type: TriggerType,
    pub workflow_id: WorkflowId,
}

/// Resolves trigger_id → Trigger reference from a slice.
/// Same pattern as WorkflowRegistry: pure lookup, no I/O.
pub struct TriggerRegistry<'a> {
    triggers: &'a [Trigger],
}

impl<'a> TriggerRegistry<'a> {
    pub fn new(triggers: &'a [Trigger]) -> Self {
        Self { triggers }
    }

    pub fn resolve(&self, trigger_id: &str) -> Option<&'a Trigger> {
        self.triggers.iter().find(|t| t.id.as_str() == trigger_id)
    }

    pub fn len(&self) -> usize {
        self.triggers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.triggers.is_empty()
    }
}

/// Evaluate which triggers should fire given a context change.
/// Pure function — no side effects, no I/O, no mutation.
///
/// Startup semantics:
/// - If previous_context is None, this is app startup.
///   DesktopStartup triggers fire, plus ProcessLaunch for current process.
/// - If previous_context is Some, only ProcessLaunch triggers evaluate.
///
/// ProcessLaunch fires when the current foreground process matches AND
/// differs from the previous foreground process (or previous was None).
pub fn evaluate_context_change(
    current_context: &str,
    previous_context: Option<&str>,
    triggers: &[Trigger],
) -> Vec<TriggerEvaluationResult> {
    triggers
        .iter()
        .filter(|t| t.enabled)
        .filter(|t| match &t.trigger_type {
            TriggerType::DesktopStartup => previous_context.is_none(),
            TriggerType::ProcessLaunch { process_name } => {
                current_context.eq_ignore_ascii_case(process_name)
                    && previous_context
                        .map(|p| !p.eq_ignore_ascii_case(process_name))
                        .unwrap_or(true)
            }
            _ => false,
        })
        .map(|t| TriggerEvaluationResult {
            trigger_id: t.id.as_str().to_owned(),
            trigger_type: t.trigger_type.clone(),
            workflow_id: t.workflow_id.clone(),
        })
        .collect()
}

/// Evaluate Manual triggers. Always returns all enabled Manual triggers.
/// Caller dispatches via execute_target() pipeline.
pub fn evaluate_manual_triggers(triggers: &[Trigger]) -> Vec<TriggerEvaluationResult> {
    triggers
        .iter()
        .filter(|t| t.enabled && matches!(t.trigger_type, TriggerType::Manual))
        .map(|t| TriggerEvaluationResult {
            trigger_id: t.id.as_str().to_owned(),
            trigger_type: t.trigger_type.clone(),
            workflow_id: t.workflow_id.clone(),
        })
        .collect()
}

/// Evaluate Time triggers. Returns enabled Time triggers whose schedule
/// matches the current minute. Called periodically by the timer thread.
///
/// Schedule format: "minute hour" (two space-separated fields).
/// - `*` matches any value
/// - Specific integers match exactly
///
/// Examples: `"0 9"` = 09:00, `"*/15 *"` = every 15 minutes, `"30 14"` = 14:30
pub fn evaluate_time_triggers(
    triggers: &[Trigger],
    current_minute: u32,
    current_hour: u32,
) -> Vec<TriggerEvaluationResult> {
    triggers
        .iter()
        .filter(|t| {
            t.enabled
                && matches!(t.trigger_type, TriggerType::Time { .. })
                && match &t.trigger_type {
                    TriggerType::Time { schedule } => {
                        schedule_matches(current_minute, current_hour, schedule)
                    }
                    _ => false,
                }
        })
        .map(|t| TriggerEvaluationResult {
            trigger_id: t.id.as_str().to_owned(),
            trigger_type: t.trigger_type.clone(),
            workflow_id: t.workflow_id.clone(),
        })
        .collect()
}

/// Check if a schedule matches the given minute and hour.
/// Pure function. Schedule format: "minute hour" with `*` for any value.
pub fn schedule_matches(minute: u32, hour: u32, schedule: &str) -> bool {
    let parts: Vec<&str> = schedule.split_whitespace().collect();
    if parts.len() != 2 {
        return false;
    }
    field_matches(minute, parts[0]) && field_matches(hour, parts[1])
}

fn field_matches(value: u32, pattern: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    pattern.parse::<u32>().map_or(false, |v| v == value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{TriggerId, TriggerVersion};

    fn trigger(id: &str, enabled: bool, tt: TriggerType, wf: &str) -> Trigger {
        Trigger {
            id: TriggerId::from_string(id),
            name: format!("Trigger {}", id),
            version: TriggerVersion::V1,
            trigger_type: tt,
            workflow_id: WorkflowId::from_string(wf),
            enabled,
        }
    }

    // ── TriggerRegistry ──

    #[test]
    fn registry_resolve_existing_trigger() {
        let triggers = vec![trigger("t1", true, TriggerType::Manual, "wf-1")];
        let reg = TriggerRegistry::new(&triggers);
        let t = reg.resolve("t1").unwrap();
        assert_eq!(t.name, "Trigger t1");
    }

    #[test]
    fn registry_resolve_missing_returns_none() {
        let triggers = vec![trigger("t1", true, TriggerType::Manual, "wf-1")];
        let reg = TriggerRegistry::new(&triggers);
        assert!(reg.resolve("nonexistent").is_none());
    }

    #[test]
    fn registry_empty() {
        let reg = TriggerRegistry::new(&[]);
        assert!(reg.resolve("anything").is_none());
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
    }

    #[test]
    fn registry_len_matches_input() {
        let triggers = vec![
            trigger("t1", true, TriggerType::Manual, "wf-1"),
            trigger("t2", true, TriggerType::Manual, "wf-2"),
        ];
        let reg = TriggerRegistry::new(&triggers);
        assert_eq!(reg.len(), 2);
        assert!(!reg.is_empty());
    }

    // ── evaluate_context_change: DesktopStartup ──

    #[test]
    fn startup_fires_desktop_startup_triggers() {
        let triggers = vec![trigger("t1", true, TriggerType::DesktopStartup, "wf-1")];
        let results = evaluate_context_change("anything.exe", None, &triggers);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].trigger_id, "t1");
        assert_eq!(results[0].workflow_id, WorkflowId::from_string("wf-1"));
    }

    #[test]
    fn startup_does_not_fire_disabled_desktop_startup() {
        let triggers = vec![trigger("t1", false, TriggerType::DesktopStartup, "wf-1")];
        let results = evaluate_context_change("anything.exe", None, &triggers);
        assert!(results.is_empty());
    }

    #[test]
    fn non_startup_does_not_fire_desktop_startup() {
        let triggers = vec![trigger("t1", true, TriggerType::DesktopStartup, "wf-1")];
        let results = evaluate_context_change("anything.exe", Some("prev.exe"), &triggers);
        assert!(results.is_empty());
    }

    // ── evaluate_context_change: ProcessLaunch ──

    #[test]
    fn startup_fires_process_launch_for_current_process() {
        let triggers = vec![trigger(
            "t1",
            true,
            TriggerType::ProcessLaunch {
                process_name: "code.exe".into(),
            },
            "wf-1",
        )];
        let results = evaluate_context_change("code.exe", None, &triggers);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].trigger_id, "t1");
    }

    #[test]
    fn process_launch_fires_on_change() {
        let triggers = vec![trigger(
            "t1",
            true,
            TriggerType::ProcessLaunch {
                process_name: "code.exe".into(),
            },
            "wf-1",
        )];
        let results = evaluate_context_change("code.exe", Some("chrome.exe"), &triggers);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn process_launch_does_not_fire_same_process() {
        let triggers = vec![trigger(
            "t1",
            true,
            TriggerType::ProcessLaunch {
                process_name: "code.exe".into(),
            },
            "wf-1",
        )];
        let results = evaluate_context_change("code.exe", Some("code.exe"), &triggers);
        assert!(results.is_empty());
    }

    #[test]
    fn process_launch_case_insensitive() {
        let triggers = vec![trigger(
            "t1",
            true,
            TriggerType::ProcessLaunch {
                process_name: "Code.exe".into(),
            },
            "wf-1",
        )];
        let results = evaluate_context_change("code.exe", None, &triggers);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn process_launch_does_not_fire_disabled() {
        let triggers = vec![trigger(
            "t1",
            false,
            TriggerType::ProcessLaunch {
                process_name: "code.exe".into(),
            },
            "wf-1",
        )];
        let results = evaluate_context_change("code.exe", None, &triggers);
        assert!(results.is_empty());
    }

    #[test]
    fn process_launch_wrong_process_does_not_fire() {
        let triggers = vec![trigger(
            "t1",
            true,
            TriggerType::ProcessLaunch {
                process_name: "code.exe".into(),
            },
            "wf-1",
        )];
        let results = evaluate_context_change("chrome.exe", None, &triggers);
        assert!(results.is_empty());
    }

    // ── evaluate_context_change: mixed triggers ──

    #[test]
    fn startup_fires_both_desktop_startup_and_process_launch() {
        let triggers = vec![
            trigger("t1", true, TriggerType::DesktopStartup, "wf-1"),
            trigger(
                "t2",
                true,
                TriggerType::ProcessLaunch {
                    process_name: "code.exe".into(),
                },
                "wf-2",
            ),
        ];
        let results = evaluate_context_change("code.exe", None, &triggers);
        assert_eq!(results.len(), 2);
        let ids: Vec<&str> = results.iter().map(|r| r.trigger_id.as_str()).collect();
        assert!(ids.contains(&"t1"));
        assert!(ids.contains(&"t2"));
    }

    #[test]
    fn non_startup_only_fires_process_launch() {
        let triggers = vec![
            trigger("t1", true, TriggerType::DesktopStartup, "wf-1"),
            trigger(
                "t2",
                true,
                TriggerType::ProcessLaunch {
                    process_name: "code.exe".into(),
                },
                "wf-2",
            ),
        ];
        let results = evaluate_context_change("code.exe", Some("chrome.exe"), &triggers);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].trigger_id, "t2");
    }

    #[test]
    fn time_triggers_never_fire_from_context_change() {
        let triggers = vec![trigger(
            "t1",
            true,
            TriggerType::Time {
                schedule: "0 9 * * *".into(),
            },
            "wf-1",
        )];
        let results = evaluate_context_change("anything.exe", None, &triggers);
        assert!(results.is_empty());
    }

    // ── evaluate_manual_triggers ──

    #[test]
    fn manual_triggers_returned() {
        let triggers = vec![trigger("t1", true, TriggerType::Manual, "wf-1")];
        let results = evaluate_manual_triggers(&triggers);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].trigger_id, "t1");
    }

    #[test]
    fn disabled_manual_triggers_not_returned() {
        let triggers = vec![trigger("t1", false, TriggerType::Manual, "wf-1")];
        let results = evaluate_manual_triggers(&triggers);
        assert!(results.is_empty());
    }

    #[test]
    fn non_manual_triggers_not_returned() {
        let triggers = vec![trigger("t1", true, TriggerType::DesktopStartup, "wf-1")];
        let results = evaluate_manual_triggers(&triggers);
        assert!(results.is_empty());
    }

    #[test]
    fn mixed_triggers_only_manual_returned() {
        let triggers = vec![
            trigger("t1", true, TriggerType::Manual, "wf-1"),
            trigger("t2", true, TriggerType::DesktopStartup, "wf-2"),
            trigger(
                "t3",
                true,
                TriggerType::ProcessLaunch {
                    process_name: "code.exe".into(),
                },
                "wf-3",
            ),
        ];
        let results = evaluate_manual_triggers(&triggers);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].trigger_id, "t1");
    }

    #[test]
    fn multiple_manual_triggers_all_returned() {
        let triggers = vec![
            trigger("t1", true, TriggerType::Manual, "wf-1"),
            trigger("t2", true, TriggerType::Manual, "wf-2"),
        ];
        let results = evaluate_manual_triggers(&triggers);
        assert_eq!(results.len(), 2);
    }

    // ── evaluate_time_triggers ──

    #[test]
    fn time_triggers_returned_when_schedule_matches() {
        let triggers = vec![trigger(
            "t1",
            true,
            TriggerType::Time {
                schedule: "30 14".into(),
            },
            "wf-1",
        )];
        let results = evaluate_time_triggers(&triggers, 30, 14);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].trigger_id, "t1");
    }

    #[test]
    fn time_triggers_not_returned_when_schedule_does_not_match() {
        let triggers = vec![trigger(
            "t1",
            true,
            TriggerType::Time {
                schedule: "30 14".into(),
            },
            "wf-1",
        )];
        let results = evaluate_time_triggers(&triggers, 0, 9);
        assert!(results.is_empty());
    }

    #[test]
    fn disabled_time_triggers_not_returned() {
        let triggers = vec![trigger(
            "t1",
            false,
            TriggerType::Time {
                schedule: "0 9 * * *".into(),
            },
            "wf-1",
        )];
        let results = evaluate_time_triggers(&triggers, 0, 9);
        assert!(results.is_empty());
    }

    #[test]
    fn non_time_triggers_not_returned() {
        let triggers = vec![trigger("t1", true, TriggerType::Manual, "wf-1")];
        let results = evaluate_time_triggers(&triggers, 0, 9);
        assert!(results.is_empty());
    }

    #[test]
    fn wildcard_schedule_matches_any_time() {
        let triggers = vec![trigger(
            "t1",
            true,
            TriggerType::Time {
                schedule: "* *".into(),
            },
            "wf-1",
        )];
        let results = evaluate_time_triggers(&triggers, 42, 7);
        assert_eq!(results.len(), 1);
    }

    // ── schedule_matches ──

    #[test]
    fn schedule_exact_match() {
        assert!(schedule_matches(0, 9, "0 9"));
        assert!(schedule_matches(30, 14, "30 14"));
        assert!(schedule_matches(59, 23, "59 23"));
    }

    #[test]
    fn schedule_minute_mismatch() {
        assert!(!schedule_matches(1, 9, "0 9"));
        assert!(!schedule_matches(31, 14, "30 14"));
    }

    #[test]
    fn schedule_hour_mismatch() {
        assert!(!schedule_matches(0, 10, "0 9"));
        assert!(!schedule_matches(30, 15, "30 14"));
    }

    #[test]
    fn schedule_wildcard_minute() {
        assert!(schedule_matches(0, 9, "* 9"));
        assert!(schedule_matches(42, 9, "* 9"));
        assert!(schedule_matches(59, 9, "* 9"));
    }

    #[test]
    fn schedule_wildcard_hour() {
        assert!(schedule_matches(0, 0, "0 *"));
        assert!(schedule_matches(0, 12, "0 *"));
        assert!(schedule_matches(0, 23, "0 *"));
    }

    #[test]
    fn schedule_both_wildcards() {
        assert!(schedule_matches(0, 0, "* *"));
        assert!(schedule_matches(42, 7, "* *"));
    }

    #[test]
    fn schedule_invalid_format() {
        assert!(!schedule_matches(0, 9, ""));
        assert!(!schedule_matches(0, 9, "0"));
        assert!(!schedule_matches(0, 9, "0 9 15"));
        assert!(!schedule_matches(0, 9, "abc def"));
    }

    #[test]
    fn schedule_non_numeric_fields() {
        assert!(!schedule_matches(0, 9, "abc 9"));
        assert!(!schedule_matches(0, 9, "0 def"));
    }
}
