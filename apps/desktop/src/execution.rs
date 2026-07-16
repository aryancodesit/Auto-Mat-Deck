use std::time::Duration;

use crate::actions::ExecutionOutcome;
use crate::agent::ACTIONS;
use crate::model::{ActionId, Workflow, WorkflowId};

const EXECUTION_TIMEOUT: Duration = Duration::from_secs(5);

/// What to execute — either a single action or an entire workflow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionTarget {
    Action(ActionId),
    Workflow(WorkflowId),
}

impl ExecutionTarget {
    pub fn action(id: ActionId) -> Self {
        Self::Action(id)
    }

    pub fn workflow(id: WorkflowId) -> Self {
        Self::Workflow(id)
    }
}

/// Resolves workflow_id → Workflow reference from a slice.
/// No registry coupling, no I/O — pure lookup over Document data.
pub struct WorkflowRegistry<'a> {
    workflows: &'a [Workflow],
}

impl<'a> WorkflowRegistry<'a> {
    pub fn new(workflows: &'a [Workflow]) -> Self {
        Self { workflows }
    }

    pub fn resolve(&self, workflow_id: &str) -> Option<&'a Workflow> {
        self.workflows.iter().find(|w| w.id.as_str() == workflow_id)
    }

    pub fn len(&self) -> usize {
        self.workflows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.workflows.is_empty()
    }
}

pub async fn execute_action(action_name: String, payload: serde_json::Value) -> ExecutionOutcome {
    match tokio::time::timeout(
        EXECUTION_TIMEOUT,
        tokio::task::spawn_blocking(move || ACTIONS.execute(&action_name, &payload)),
    )
    .await
    {
        Ok(Ok(Ok(_))) => ExecutionOutcome::Success,
        Ok(Ok(Err(e))) => {
            if e.message.starts_with("Unknown action:") {
                ExecutionOutcome::ActionNotFound
            } else {
                ExecutionOutcome::Failed(e.message)
            }
        }
        Ok(Err(_)) => ExecutionOutcome::Panicked,
        Err(_) => ExecutionOutcome::Timeout,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{WorkflowStep, WorkflowVersion};
    use serde_json::json;

    // ── execute_action (Sprint 4, preserved) ──

    #[tokio::test]
    async fn execute_action_with_valid_action_returns_success() {
        let result = execute_action("lock".into(), json!({})).await;
        assert!(matches!(result, ExecutionOutcome::Success));
    }

    #[tokio::test]
    async fn execute_action_with_unknown_action_returns_action_not_found() {
        let result = execute_action("nonexistent_action".into(), json!({})).await;
        assert!(matches!(result, ExecutionOutcome::ActionNotFound));
    }

    #[tokio::test]
    async fn execute_action_with_invalid_payload_returns_failed() {
        let result = execute_action("launch".into(), json!({})).await;
        assert!(matches!(result, ExecutionOutcome::Failed(_)));
    }

    // ── ExecutionTarget ──

    #[test]
    fn execution_target_action_variant() {
        let t = ExecutionTarget::action(ActionId::from_string("launch"));
        assert_eq!(t, ExecutionTarget::Action(ActionId::from_string("launch")));
    }

    #[test]
    fn execution_target_workflow_variant() {
        let t = ExecutionTarget::workflow(WorkflowId::from_string("wf-1"));
        assert_eq!(
            t,
            ExecutionTarget::Workflow(WorkflowId::from_string("wf-1"))
        );
    }

    #[test]
    fn execution_target_action_not_equal_to_workflow() {
        let a = ExecutionTarget::action(ActionId::from_string("x"));
        let w = ExecutionTarget::workflow(WorkflowId::from_string("x"));
        assert_ne!(a, w);
    }

    // ── WorkflowRegistry ──

    fn make_workflows() -> Vec<Workflow> {
        vec![
            Workflow {
                id: WorkflowId::from_string("wf-1"),
                name: "Workflow One".into(),
                version: WorkflowVersion::V1,
                steps: vec![WorkflowStep {
                    action_id: ActionId::from_string("lock"),
                    payload: json!(null),
                }],
                enabled: true,
            },
            Workflow {
                id: WorkflowId::from_string("wf-2"),
                name: "Workflow Two".into(),
                version: WorkflowVersion::V1,
                steps: vec![
                    WorkflowStep {
                        action_id: ActionId::from_string("launch"),
                        payload: json!({"app": "chrome"}),
                    },
                    WorkflowStep {
                        action_id: ActionId::from_string("lock"),
                        payload: json!(null),
                    },
                ],
                enabled: false,
            },
        ]
    }

    #[test]
    fn registry_resolve_existing_workflow() {
        let workflows = make_workflows();
        let reg = WorkflowRegistry::new(&workflows);
        let w = reg.resolve("wf-1").unwrap();
        assert_eq!(w.name, "Workflow One");
        assert_eq!(w.steps.len(), 1);
    }

    #[test]
    fn registry_resolve_second_workflow() {
        let workflows = make_workflows();
        let reg = WorkflowRegistry::new(&workflows);
        let w = reg.resolve("wf-2").unwrap();
        assert_eq!(w.name, "Workflow Two");
        assert_eq!(w.steps.len(), 2);
        assert!(!w.enabled);
    }

    #[test]
    fn registry_resolve_missing_returns_none() {
        let workflows = make_workflows();
        let reg = WorkflowRegistry::new(&workflows);
        assert!(reg.resolve("nonexistent").is_none());
    }

    #[test]
    fn registry_empty_workflows() {
        let reg = WorkflowRegistry::new(&[]);
        assert!(reg.resolve("anything").is_none());
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
    }

    #[test]
    fn registry_len_matches_input() {
        let workflows = make_workflows();
        let reg = WorkflowRegistry::new(&workflows);
        assert_eq!(reg.len(), 2);
        assert!(!reg.is_empty());
    }
}
