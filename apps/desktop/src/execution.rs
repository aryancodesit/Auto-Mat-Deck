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

/// Unified dispatch: resolve + execute in one call.
/// Encapsulates workflow registry lookup so agent.rs stays a thin coordinator.
pub async fn execute_target(
    target: &ExecutionTarget,
    action_name: &str,
    payload: serde_json::Value,
    workflows: &[Workflow],
) -> ControlInvokeResultDto {
    match target {
        ExecutionTarget::Action(_) => {
            let outcome = execute_action(action_name.to_owned(), payload).await;
            ControlInvokeResultDto::from_action_outcome(outcome)
        }
        ExecutionTarget::Workflow(workflow_id) => {
            let registry = WorkflowRegistry::new(workflows);
            match registry.resolve(workflow_id.as_str()) {
                Some(workflow) => {
                    let result = execute_workflow(workflow).await;
                    ControlInvokeResultDto::from_workflow_result(result)
                }
                None => ControlInvokeResultDto::not_found("unknown_workflow"),
            }
        }
    }
}

/// Transport DTO for control_invoke_result wire protocol.
/// Keeps runtime models decoupled from serialization.
#[derive(Debug, Clone)]
pub struct ControlInvokeResultDto {
    pub accepted: bool,
    pub executed: Option<bool>,
    pub reason: Option<String>,
    pub execution_error: Option<String>,
    pub steps: Vec<ControlInvokeStepDto>,
}

#[derive(Debug, Clone)]
pub struct ControlInvokeStepDto {
    pub step_index: usize,
    pub action_id: String,
    pub executed: bool,
    pub error: Option<String>,
}

impl ControlInvokeResultDto {
    pub fn from_action_outcome(outcome: ExecutionOutcome) -> Self {
        match outcome {
            ExecutionOutcome::Success => Self {
                accepted: true,
                executed: Some(true),
                reason: None,
                execution_error: None,
                steps: Vec::new(),
            },
            ExecutionOutcome::Failed(msg) => Self {
                accepted: true,
                executed: Some(false),
                reason: None,
                execution_error: Some(msg),
                steps: Vec::new(),
            },
            ExecutionOutcome::ActionNotFound => Self {
                accepted: true,
                executed: Some(false),
                reason: None,
                execution_error: Some("action_not_found".into()),
                steps: Vec::new(),
            },
            ExecutionOutcome::Timeout => Self {
                accepted: true,
                executed: Some(false),
                reason: None,
                execution_error: Some("execution_timeout".into()),
                steps: Vec::new(),
            },
            ExecutionOutcome::Panicked => Self {
                accepted: true,
                executed: Some(false),
                reason: None,
                execution_error: Some("execution_panicked".into()),
                steps: Vec::new(),
            },
        }
    }

    pub fn from_workflow_result(result: WorkflowExecutionResult) -> Self {
        let steps = result
            .steps
            .into_iter()
            .map(|s| ControlInvokeStepDto {
                step_index: s.step_index,
                action_id: s.action_id.as_str().to_owned(),
                executed: s.executed,
                error: s.error,
            })
            .collect();
        Self {
            accepted: result.accepted,
            executed: if result.accepted {
                Some(result.executed)
            } else {
                None
            },
            reason: result.reason,
            execution_error: result.execution_error,
            steps,
        }
    }

    pub fn not_found(reason: &str) -> Self {
        Self {
            accepted: false,
            executed: None,
            reason: Some(reason.into()),
            execution_error: None,
            steps: Vec::new(),
        }
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

    // ── execute_target: action dispatch (v0.5 regression) ──

    #[tokio::test]
    async fn target_action_success_delegates_to_execute_action() {
        let target = ExecutionTarget::action(ActionId::from_string("lock"));
        let dto = execute_target(&target, "lock", json!(null), &[]).await;
        assert!(dto.accepted);
        assert_eq!(dto.executed, Some(true));
        assert!(dto.execution_error.is_none());
        assert!(dto.steps.is_empty());
    }

    #[tokio::test]
    async fn target_action_not_found_returns_action_not_found() {
        let target = ExecutionTarget::action(ActionId::from_string("nonexistent"));
        let dto = execute_target(&target, "nonexistent", json!(null), &[]).await;
        assert!(dto.accepted);
        assert_eq!(dto.executed, Some(false));
        assert_eq!(dto.execution_error.as_deref(), Some("action_not_found"));
    }

    // ── execute_target: workflow dispatch ──

    #[tokio::test]
    async fn target_workflow_success_delegates_to_execute_workflow() {
        let wf = enabled_workflow("wf-dispatch", vec![lock_step()]);
        let target = ExecutionTarget::workflow(WorkflowId::from_string("wf-dispatch"));
        let dto = execute_target(&target, "lock", json!(null), &[wf]).await;
        assert!(dto.accepted);
        assert_eq!(dto.executed, Some(true));
        assert_eq!(dto.steps.len(), 1);
        assert!(dto.steps[0].executed);
    }

    #[tokio::test]
    async fn target_workflow_disabled_rejected() {
        let wf = disabled_workflow("wf-dis-disp");
        let target = ExecutionTarget::workflow(WorkflowId::from_string("wf-dis-disp"));
        let dto = execute_target(&target, "lock", json!(null), &[wf]).await;
        assert!(!dto.accepted);
        assert_eq!(dto.reason.as_deref(), Some("workflow_disabled"));
        assert_eq!(dto.executed, None);
    }

    #[tokio::test]
    async fn target_workflow_not_in_registry_rejected() {
        let target = ExecutionTarget::workflow(WorkflowId::from_string("wf-nonexistent"));
        let dto = execute_target(&target, "lock", json!(null), &[]).await;
        assert!(!dto.accepted);
        assert_eq!(dto.reason.as_deref(), Some("unknown_workflow"));
        assert_eq!(dto.executed, None);
    }

    // ── execute_target: action_name and payload passed through ──

    #[tokio::test]
    async fn target_action_invalid_payload_returns_failed() {
        let target = ExecutionTarget::action(ActionId::from_string("launch"));
        let dto = execute_target(&target, "launch", json!(null), &[]).await;
        assert!(dto.accepted);
        assert_eq!(dto.executed, Some(false));
        assert!(dto.execution_error.is_some());
    }

    // ── ControlInvokeResultDto: action outcome mapping ──

    #[test]
    fn dto_action_success() {
        let dto = ControlInvokeResultDto::from_action_outcome(ExecutionOutcome::Success);
        assert!(dto.accepted);
        assert_eq!(dto.executed, Some(true));
        assert!(dto.execution_error.is_none());
        assert!(dto.steps.is_empty());
    }

    #[test]
    fn dto_action_not_found() {
        let dto = ControlInvokeResultDto::from_action_outcome(ExecutionOutcome::ActionNotFound);
        assert!(dto.accepted);
        assert_eq!(dto.executed, Some(false));
        assert_eq!(dto.execution_error.as_deref(), Some("action_not_found"));
    }

    #[test]
    fn dto_action_timeout() {
        let dto = ControlInvokeResultDto::from_action_outcome(ExecutionOutcome::Timeout);
        assert!(dto.accepted);
        assert_eq!(dto.executed, Some(false));
        assert_eq!(dto.execution_error.as_deref(), Some("execution_timeout"));
    }

    #[test]
    fn dto_action_panicked() {
        let dto = ControlInvokeResultDto::from_action_outcome(ExecutionOutcome::Panicked);
        assert!(dto.accepted);
        assert_eq!(dto.executed, Some(false));
        assert_eq!(dto.execution_error.as_deref(), Some("execution_panicked"));
    }

    #[test]
    fn dto_action_failed_with_message() {
        let dto = ControlInvokeResultDto::from_action_outcome(ExecutionOutcome::Failed(
            "bad payload".into(),
        ));
        assert!(dto.accepted);
        assert_eq!(dto.executed, Some(false));
        assert_eq!(dto.execution_error.as_deref(), Some("bad payload"));
    }

    // ── ControlInvokeResultDto: workflow result mapping ──

    #[tokio::test]
    async fn dto_workflow_success() {
        let wf = enabled_workflow("wf-dto", vec![lock_step()]);
        let rt = execute_workflow(&wf).await;
        let dto = ControlInvokeResultDto::from_workflow_result(rt);
        assert!(dto.accepted);
        assert_eq!(dto.executed, Some(true));
        assert_eq!(dto.steps.len(), 1);
        assert!(dto.steps[0].executed);
        assert_eq!(dto.steps[0].action_id, "lock");
    }

    #[tokio::test]
    async fn dto_workflow_disabled() {
        let wf = disabled_workflow("wf-dto-dis");
        let rt = execute_workflow(&wf).await;
        let dto = ControlInvokeResultDto::from_workflow_result(rt);
        assert!(!dto.accepted);
        assert_eq!(dto.executed, None);
        assert_eq!(dto.reason.as_deref(), Some("workflow_disabled"));
    }

    // ── ControlInvokeResultDto::not_found ──

    #[test]
    fn dto_not_found() {
        let dto = ControlInvokeResultDto::not_found("unknown_button");
        assert!(!dto.accepted);
        assert_eq!(dto.executed, None);
        assert_eq!(dto.reason.as_deref(), Some("unknown_button"));
        assert!(dto.execution_error.is_none());
        assert!(dto.steps.is_empty());
    }
}
