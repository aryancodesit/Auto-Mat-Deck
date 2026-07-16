use crate::model::{Workflow, WorkflowVersion};

/// Errors from structural validation of workflow data.
/// These operate only on serialized data — no registry or execution dependencies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StructuralError {
    EmptyId,
    EmptyName,
    EmptySteps,
    EmptyActionId { step_index: usize },
    UnsupportedVersion(u16),
}

impl std::fmt::Display for StructuralError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyId => write!(f, "workflow id is empty"),
            Self::EmptyName => write!(f, "workflow name is empty"),
            Self::EmptySteps => write!(f, "workflow has no steps"),
            Self::EmptyActionId { step_index } => {
                write!(f, "step {} has empty action_id", step_index)
            }
            Self::UnsupportedVersion(v) => write!(f, "unsupported workflow version: {}", v),
        }
    }
}

/// Structural validation: operates only on serialized workflow data.
/// No I/O, no runtime state, no registry lookups.
pub fn validate_structural(workflow: &Workflow) -> Result<(), StructuralError> {
    if workflow.id.as_str().is_empty() {
        return Err(StructuralError::EmptyId);
    }
    if workflow.name.is_empty() {
        return Err(StructuralError::EmptyName);
    }
    if workflow.steps.is_empty() {
        return Err(StructuralError::EmptySteps);
    }
    if workflow.version != WorkflowVersion::V1 {
        return Err(StructuralError::UnsupportedVersion(workflow.version.0));
    }
    for (i, step) in workflow.steps.iter().enumerate() {
        if step.action_id.as_str().is_empty() {
            return Err(StructuralError::EmptyActionId { step_index: i });
        }
    }
    Ok(())
}

/// Check for duplicate workflow IDs within a slice.
/// Returns the index of the first duplicate, if any.
pub fn find_duplicate_workflow_ids(workflows: &[Workflow]) -> Option<usize> {
    for i in 1..workflows.len() {
        let id = workflows[i].id.as_str();
        if workflows[..i].iter().any(|w| w.id.as_str() == id) {
            return Some(i);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ActionId, WorkflowId, WorkflowStep, WorkflowVersion};
    use serde_json::json;

    fn make_workflow(id: &str, name: &str, step_count: usize) -> Workflow {
        Workflow {
            id: WorkflowId::from_string(id),
            name: name.into(),
            version: WorkflowVersion::V1,
            steps: (0..step_count)
                .map(|i| WorkflowStep {
                    action_id: ActionId::from_string(format!("action-{}", i)),
                    payload: json!(null),
                })
                .collect(),
            enabled: true,
        }
    }

    // ── Structural validation: happy path ──

    #[test]
    fn valid_workflow_passes_validation() {
        let w = make_workflow("w1", "Test Workflow", 2);
        assert_eq!(validate_structural(&w), Ok(()));
    }

    // ── Structural validation: empty ID ──

    #[test]
    fn empty_id_rejected() {
        let mut w = make_workflow("w1", "Test", 1);
        w.id = WorkflowId::from_string("");
        assert_eq!(validate_structural(&w), Err(StructuralError::EmptyId));
    }

    // ── Structural validation: empty name ──

    #[test]
    fn empty_name_rejected() {
        let mut w = make_workflow("w1", "Test", 1);
        w.name = String::new();
        assert_eq!(validate_structural(&w), Err(StructuralError::EmptyName));
    }

    // ── Structural validation: empty steps ──

    #[test]
    fn empty_steps_rejected() {
        let w = make_workflow("w1", "Test", 0);
        assert_eq!(validate_structural(&w), Err(StructuralError::EmptySteps));
    }

    // ── Structural validation: unsupported version ──

    #[test]
    fn unsupported_version_rejected() {
        let mut w = make_workflow("w1", "Test", 1);
        w.version = WorkflowVersion(99);
        assert_eq!(
            validate_structural(&w),
            Err(StructuralError::UnsupportedVersion(99))
        );
    }

    // ── Structural validation: empty action_id in step ──

    #[test]
    fn empty_action_id_in_first_step_rejected() {
        let mut w = make_workflow("w1", "Test", 2);
        w.steps[0].action_id = ActionId::from_string("");
        assert_eq!(
            validate_structural(&w),
            Err(StructuralError::EmptyActionId { step_index: 0 })
        );
    }

    #[test]
    fn empty_action_id_in_second_step_rejected() {
        let mut w = make_workflow("w1", "Test", 2);
        w.steps[1].action_id = ActionId::from_string("");
        assert_eq!(
            validate_structural(&w),
            Err(StructuralError::EmptyActionId { step_index: 1 })
        );
    }

    // ── Duplicate ID detection ──

    #[test]
    fn no_duplicates_returns_none() {
        let workflows = vec![
            make_workflow("w1", "A", 1),
            make_workflow("w2", "B", 1),
            make_workflow("w3", "C", 1),
        ];
        assert_eq!(find_duplicate_workflow_ids(&workflows), None);
    }

    #[test]
    fn duplicate_at_index_1_detected() {
        let workflows = vec![
            make_workflow("w1", "A", 1),
            make_workflow("w1", "B", 1),
            make_workflow("w2", "C", 1),
        ];
        assert_eq!(find_duplicate_workflow_ids(&workflows), Some(1));
    }

    #[test]
    fn duplicate_at_index_2_detected() {
        let workflows = vec![
            make_workflow("w1", "A", 1),
            make_workflow("w2", "B", 1),
            make_workflow("w1", "C", 1),
        ];
        assert_eq!(find_duplicate_workflow_ids(&workflows), Some(2));
    }

    #[test]
    fn empty_workflows_no_duplicates() {
        assert_eq!(find_duplicate_workflow_ids(&[]), None);
    }

    // ── Serialization round-trip ──

    #[test]
    fn workflow_serialization_round_trip() {
        let w = make_workflow("w1", "Test Workflow", 2);
        let json = serde_json::to_string(&w).unwrap();
        let back: Workflow = serde_json::from_str(&json).unwrap();
        assert_eq!(w, back);
    }

    #[test]
    fn workflow_version_v1_serializes() {
        let v = WorkflowVersion::V1;
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, "1");
    }

    #[test]
    fn workflow_step_with_payload_round_trip() {
        let step = WorkflowStep {
            action_id: ActionId::from_string("launch"),
            payload: json!({"app": "chrome"}),
        };
        let json = serde_json::to_string(&step).unwrap();
        let back: WorkflowStep = serde_json::from_str(&json).unwrap();
        assert_eq!(step, back);
    }

    #[test]
    fn workflow_step_without_payload_defaults_to_null() {
        let json = r#"{"action_id":"test"}"#;
        let step: WorkflowStep = serde_json::from_str(json).unwrap();
        assert_eq!(step.payload, json!(null));
    }

    // ── Document.workflows backward compatibility ──

    #[test]
    fn document_without_workflows_field_deserializes() {
        let json = r#"{
            "schema": 1,
            "created_with": "0.5.0",
            "last_saved_with": "0.5.0",
            "devices": [],
            "profiles": []
        }"#;
        let doc: crate::model::Document = serde_json::from_str(json).unwrap();
        assert_eq!(doc.workflows, vec![]);
    }

    #[test]
    fn document_round_trip_with_workflows() {
        let mut doc = crate::model::Document::empty();
        doc.workflows.push(make_workflow("w1", "My Workflow", 1));
        let json = serde_json::to_string(&doc).unwrap();
        let back: crate::model::Document = serde_json::from_str(&json).unwrap();
        assert_eq!(doc, back);
    }

    // ── Error display ──

    #[test]
    fn error_display_messages() {
        assert_eq!(StructuralError::EmptyId.to_string(), "workflow id is empty");
        assert_eq!(
            StructuralError::EmptyName.to_string(),
            "workflow name is empty"
        );
        assert_eq!(
            StructuralError::EmptySteps.to_string(),
            "workflow has no steps"
        );
        assert_eq!(
            StructuralError::EmptyActionId { step_index: 2 }.to_string(),
            "step 2 has empty action_id"
        );
        assert_eq!(
            StructuralError::UnsupportedVersion(99).to_string(),
            "unsupported workflow version: 99"
        );
    }
}
