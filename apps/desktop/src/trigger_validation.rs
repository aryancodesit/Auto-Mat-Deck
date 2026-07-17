use crate::model::{Trigger, TriggerVersion};

/// Errors from structural validation of trigger data.
/// These operate only on serialized data — no registry or execution dependencies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TriggerStructuralError {
    EmptyId,
    EmptyName,
    EmptyWorkflowId,
    MissingSchedule,
    UnsupportedVersion(u16),
}

impl std::fmt::Display for TriggerStructuralError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyId => write!(f, "trigger id is empty"),
            Self::EmptyName => write!(f, "trigger name is empty"),
            Self::EmptyWorkflowId => write!(f, "trigger workflow_id is empty"),
            Self::MissingSchedule => write!(f, "time trigger has empty schedule"),
            Self::UnsupportedVersion(v) => write!(f, "unsupported trigger version: {}", v),
        }
    }
}

/// Structural validation: operates only on serialized trigger data.
/// No I/O, no runtime state, no registry lookups.
pub fn validate_structural(trigger: &Trigger) -> Result<(), TriggerStructuralError> {
    if trigger.id.as_str().is_empty() {
        return Err(TriggerStructuralError::EmptyId);
    }
    if trigger.name.is_empty() {
        return Err(TriggerStructuralError::EmptyName);
    }
    if trigger.workflow_id.as_str().is_empty() {
        return Err(TriggerStructuralError::EmptyWorkflowId);
    }
    if trigger.version != TriggerVersion::V1 {
        return Err(TriggerStructuralError::UnsupportedVersion(
            trigger.version.0,
        ));
    }
    if let crate::model::TriggerType::Time { ref schedule } = trigger.trigger_type
        && schedule.is_empty()
    {
        return Err(TriggerStructuralError::MissingSchedule);
    }
    Ok(())
}

/// Check for duplicate trigger IDs within a slice.
/// Returns the index of the first duplicate, if any.
pub fn find_duplicate_trigger_ids(triggers: &[Trigger]) -> Option<usize> {
    for i in 1..triggers.len() {
        let id = triggers[i].id.as_str();
        if triggers[..i].iter().any(|t| t.id.as_str() == id) {
            return Some(i);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{TriggerId, TriggerType, TriggerVersion, WorkflowId};

    fn make_trigger(id: &str, name: &str, wf_id: &str) -> Trigger {
        Trigger {
            id: TriggerId::from_string(id),
            name: name.into(),
            version: TriggerVersion::V1,
            trigger_type: TriggerType::Manual,
            workflow_id: WorkflowId::from_string(wf_id),
            enabled: true,
        }
    }

    // ── Structural validation: happy path ──

    #[test]
    fn valid_trigger_passes_validation() {
        let t = make_trigger("t1", "My Trigger", "w1");
        assert_eq!(validate_structural(&t), Ok(()));
    }

    #[test]
    fn valid_time_trigger_passes() {
        let mut t = make_trigger("t1", "Scheduled", "w1");
        t.trigger_type = TriggerType::Time {
            schedule: "0 9 * * *".into(),
        };
        assert_eq!(validate_structural(&t), Ok(()));
    }

    #[test]
    fn valid_process_launch_trigger_passes() {
        let mut t = make_trigger("t1", "On Chrome", "w1");
        t.trigger_type = TriggerType::ProcessLaunch {
            process_name: "chrome.exe".into(),
        };
        assert_eq!(validate_structural(&t), Ok(()));
    }

    #[test]
    fn valid_desktop_startup_trigger_passes() {
        let mut t = make_trigger("t1", "On Boot", "w1");
        t.trigger_type = TriggerType::DesktopStartup;
        assert_eq!(validate_structural(&t), Ok(()));
    }

    // ── Structural validation: errors ──

    #[test]
    fn empty_id_rejected() {
        let mut t = make_trigger("t1", "Test", "w1");
        t.id = TriggerId::from_string("");
        assert_eq!(
            validate_structural(&t),
            Err(TriggerStructuralError::EmptyId)
        );
    }

    #[test]
    fn empty_name_rejected() {
        let mut t = make_trigger("t1", "Test", "w1");
        t.name = String::new();
        assert_eq!(
            validate_structural(&t),
            Err(TriggerStructuralError::EmptyName)
        );
    }

    #[test]
    fn empty_workflow_id_rejected() {
        let mut t = make_trigger("t1", "Test", "w1");
        t.workflow_id = WorkflowId::from_string("");
        assert_eq!(
            validate_structural(&t),
            Err(TriggerStructuralError::EmptyWorkflowId)
        );
    }

    #[test]
    fn empty_time_schedule_rejected() {
        let mut t = make_trigger("t1", "Scheduled", "w1");
        t.trigger_type = TriggerType::Time {
            schedule: String::new(),
        };
        assert_eq!(
            validate_structural(&t),
            Err(TriggerStructuralError::MissingSchedule)
        );
    }

    #[test]
    fn unsupported_version_rejected() {
        let mut t = make_trigger("t1", "Test", "w1");
        t.version = TriggerVersion(99);
        assert_eq!(
            validate_structural(&t),
            Err(TriggerStructuralError::UnsupportedVersion(99))
        );
    }

    // ── Duplicate ID detection ──

    #[test]
    fn no_duplicates_returns_none() {
        let triggers = vec![
            make_trigger("t1", "A", "w1"),
            make_trigger("t2", "B", "w1"),
            make_trigger("t3", "C", "w1"),
        ];
        assert_eq!(find_duplicate_trigger_ids(&triggers), None);
    }

    #[test]
    fn duplicate_at_index_1_detected() {
        let triggers = vec![
            make_trigger("t1", "A", "w1"),
            make_trigger("t1", "B", "w1"),
            make_trigger("t2", "C", "w1"),
        ];
        assert_eq!(find_duplicate_trigger_ids(&triggers), Some(1));
    }

    #[test]
    fn duplicate_at_index_2_detected() {
        let triggers = vec![
            make_trigger("t1", "A", "w1"),
            make_trigger("t2", "B", "w1"),
            make_trigger("t1", "C", "w1"),
        ];
        assert_eq!(find_duplicate_trigger_ids(&triggers), Some(2));
    }

    #[test]
    fn empty_triggers_no_duplicates() {
        assert_eq!(find_duplicate_trigger_ids(&[]), None);
    }

    // ── Serialization round-trip ──

    #[test]
    fn trigger_serialization_round_trip() {
        let t = make_trigger("t1", "My Trigger", "w1");
        let json = serde_json::to_string(&t).unwrap();
        let back: Trigger = serde_json::from_str(&json).unwrap();
        assert_eq!(t, back);
    }

    #[test]
    fn trigger_version_v1_serializes() {
        let v = TriggerVersion::V1;
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, "1");
    }

    #[test]
    fn time_trigger_serialization_round_trip() {
        let mut t = make_trigger("t1", "Scheduled", "w1");
        t.trigger_type = TriggerType::Time {
            schedule: "0 9 * * *".into(),
        };
        let json = serde_json::to_string(&t).unwrap();
        let back: Trigger = serde_json::from_str(&json).unwrap();
        assert_eq!(t, back);
    }

    #[test]
    fn process_launch_trigger_serialization_round_trip() {
        let mut t = make_trigger("t1", "On Chrome", "w1");
        t.trigger_type = TriggerType::ProcessLaunch {
            process_name: "chrome.exe".into(),
        };
        let json = serde_json::to_string(&t).unwrap();
        let back: Trigger = serde_json::from_str(&json).unwrap();
        assert_eq!(t, back);
    }

    #[test]
    fn manual_trigger_serializes_as_expected() {
        let t = make_trigger("t1", "Manual", "w1");
        let json = serde_json::to_string(&t).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["type"], "Manual");
    }

    #[test]
    fn time_trigger_type_serializes_with_schedule() {
        let mut t = make_trigger("t1", "Scheduled", "w1");
        t.trigger_type = TriggerType::Time {
            schedule: "0 9 * * *".into(),
        };
        let json = serde_json::to_string(&t).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["type"], "Time");
        assert_eq!(parsed["schedule"], "0 9 * * *");
    }

    // ── Document.triggers backward compatibility ──

    #[test]
    fn document_without_triggers_field_deserializes() {
        let json = r#"{
            "schema": 1,
            "created_with": "0.6.0",
            "last_saved_with": "0.6.0",
            "devices": [],
            "profiles": []
        }"#;
        let doc: crate::model::Document = serde_json::from_str(json).unwrap();
        assert_eq!(doc.triggers, vec![]);
    }

    #[test]
    fn document_round_trip_with_triggers() {
        let mut doc = crate::model::Document::empty();
        doc.triggers.push(make_trigger("t1", "My Trigger", "w1"));
        let json = serde_json::to_string(&doc).unwrap();
        let back: crate::model::Document = serde_json::from_str(&json).unwrap();
        assert_eq!(doc, back);
    }

    // ── Error display ──

    #[test]
    fn error_display_messages() {
        assert_eq!(
            TriggerStructuralError::EmptyId.to_string(),
            "trigger id is empty"
        );
        assert_eq!(
            TriggerStructuralError::EmptyName.to_string(),
            "trigger name is empty"
        );
        assert_eq!(
            TriggerStructuralError::EmptyWorkflowId.to_string(),
            "trigger workflow_id is empty"
        );
        assert_eq!(
            TriggerStructuralError::MissingSchedule.to_string(),
            "time trigger has empty schedule"
        );
        assert_eq!(
            TriggerStructuralError::UnsupportedVersion(99).to_string(),
            "unsupported trigger version: 99"
        );
    }
}
