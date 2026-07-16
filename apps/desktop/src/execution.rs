use std::time::Duration;

use crate::actions::ExecutionOutcome;
use crate::agent::ACTIONS;
use crate::model::{ActionId, StepResult, Workflow, WorkflowExecutionResult, WorkflowId};

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

/// Execute a workflow sequentially. Each step delegates to execute_action().
/// Stops immediately on first failure. No retries, no rollback, no parallelism.
pub async fn execute_workflow(workflow: &Workflow) -> WorkflowExecutionResult {
    let mut result = WorkflowExecutionResult {
        workflow_id: workflow.id.clone(),
        accepted: true,
        reason: None,
        executed: false,
        steps: Vec::with_capacity(workflow.steps.len()),
        execution_error: None,
    };

    if !workflow.enabled {
        result.accepted = false;
        result.reason = Some("workflow_disabled".into());
        return result;
    }

    for (i, step) in workflow.steps.iter().enumerate() {
        let outcome =
            execute_action(step.action_id.as_str().to_owned(), step.payload.clone()).await;

        let step_result = StepResult {
            step_index: i,
            action_id: step.action_id.clone(),
            executed: matches!(outcome, ExecutionOutcome::Success),
            error: match &outcome {
                ExecutionOutcome::Success => None,
                ExecutionOutcome::Failed(msg) => Some(msg.clone()),
                ExecutionOutcome::ActionNotFound => Some("action_not_found".into()),
                ExecutionOutcome::Timeout => Some("execution_timeout".into()),
                ExecutionOutcome::Panicked => Some("execution_panicked".into()),
            },
        };

        let failed = !step_result.executed;
        result.steps.push(step_result);

        if failed {
            result.execution_error = result.steps.last().unwrap().error.clone();
            return result;
        }
    }

    result.executed = true;
    result
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

    // ── execute_workflow ──

    fn enabled_workflow(id: &str, steps: Vec<WorkflowStep>) -> Workflow {
        Workflow {
            id: WorkflowId::from_string(id),
            name: format!("Workflow {}", id),
            version: WorkflowVersion::V1,
            steps,
            enabled: true,
        }
    }

    fn disabled_workflow(id: &str) -> Workflow {
        Workflow {
            id: WorkflowId::from_string(id),
            name: format!("Workflow {}", id),
            version: WorkflowVersion::V1,
            steps: vec![WorkflowStep {
                action_id: ActionId::from_string("lock"),
                payload: json!(null),
            }],
            enabled: false,
        }
    }

    fn lock_step() -> WorkflowStep {
        WorkflowStep {
            action_id: ActionId::from_string("lock"),
            payload: json!(null),
        }
    }

    fn failing_step() -> WorkflowStep {
        WorkflowStep {
            action_id: ActionId::from_string("nonexistent_action"),
            payload: json!(null),
        }
    }

    // ── Single-step success ──

    #[tokio::test]
    async fn workflow_single_step_success() {
        let wf = enabled_workflow("wf-s1", vec![lock_step()]);
        let result = execute_workflow(&wf).await;
        assert!(result.accepted);
        assert!(result.executed);
        assert!(result.execution_error.is_none());
        assert_eq!(result.steps.len(), 1);
        assert!(result.steps[0].executed);
        assert!(result.steps[0].error.is_none());
    }

    // ── Multi-step success ──

    #[tokio::test]
    async fn workflow_multi_step_success() {
        let wf = enabled_workflow("wf-ms", vec![lock_step(), lock_step(), lock_step()]);
        let result = execute_workflow(&wf).await;
        assert!(result.accepted);
        assert!(result.executed);
        assert_eq!(result.steps.len(), 3);
        assert!(result.steps.iter().all(|s| s.executed));
    }

    // ── First-step failure ──

    #[tokio::test]
    async fn workflow_first_step_failure_stops() {
        let wf = enabled_workflow("wf-f1", vec![failing_step(), lock_step()]);
        let result = execute_workflow(&wf).await;
        assert!(result.accepted);
        assert!(!result.executed);
        assert_eq!(result.steps.len(), 1, "second step must not execute");
        assert!(!result.steps[0].executed);
        assert_eq!(result.steps[0].error.as_deref(), Some("action_not_found"));
        assert_eq!(result.execution_error.as_deref(), Some("action_not_found"));
    }

    // ── Middle-step failure ──

    #[tokio::test]
    async fn workflow_middle_step_failure_stops() {
        let wf = enabled_workflow("wf-mid", vec![lock_step(), failing_step(), lock_step()]);
        let result = execute_workflow(&wf).await;
        assert!(result.accepted);
        assert!(!result.executed);
        assert_eq!(result.steps.len(), 2, "third step must not execute");
        assert!(result.steps[0].executed);
        assert!(!result.steps[1].executed);
    }

    // ── Last-step failure ──

    #[tokio::test]
    async fn workflow_last_step_failure() {
        let wf = enabled_workflow("wf-last", vec![lock_step(), lock_step(), failing_step()]);
        let result = execute_workflow(&wf).await;
        assert!(result.accepted);
        assert!(!result.executed);
        assert_eq!(result.steps.len(), 3);
        assert!(result.steps[0].executed);
        assert!(result.steps[1].executed);
        assert!(!result.steps[2].executed);
    }

    // ── Disabled workflow ──

    #[tokio::test]
    async fn workflow_disabled_rejected() {
        let wf = disabled_workflow("wf-dis");
        let result = execute_workflow(&wf).await;
        assert!(!result.accepted);
        assert_eq!(result.reason.as_deref(), Some("workflow_disabled"));
        assert!(!result.executed);
        assert!(result.steps.is_empty());
    }

    // ── Result aggregation: step indices preserved ──

    #[tokio::test]
    async fn workflow_step_indices_preserved() {
        let wf = enabled_workflow("wf-idx", vec![lock_step(), lock_step()]);
        let result = execute_workflow(&wf).await;
        assert_eq!(result.steps[0].step_index, 0);
        assert_eq!(result.steps[1].step_index, 1);
    }

    // ── Result aggregation: action_ids preserved ──

    #[tokio::test]
    async fn workflow_step_action_ids_preserved() {
        let wf = enabled_workflow("wf-aids", vec![lock_step(), lock_step()]);
        let result = execute_workflow(&wf).await;
        assert_eq!(result.steps[0].action_id, ActionId::from_string("lock"));
        assert_eq!(result.steps[1].action_id, ActionId::from_string("lock"));
    }

    // ── Result aggregation: workflow_id propagated ──

    #[tokio::test]
    async fn workflow_result_contains_workflow_id() {
        let wf = enabled_workflow("wf-id-check", vec![lock_step()]);
        let result = execute_workflow(&wf).await;
        assert_eq!(result.workflow_id, WorkflowId::from_string("wf-id-check"));
    }
}
