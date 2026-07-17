use crate::execution;
use crate::model::{ExecutionTarget, Workflow};
use crate::trigger_execution::TriggerEvaluationResult;

/// Bridges sync observer/timer threads to async execute_target() pipeline.
/// Creates its own tokio runtime — call from non-async context only.
pub struct TriggerDispatcher {
    runtime: tokio::runtime::Runtime,
}

impl TriggerDispatcher {
    pub fn new() -> Self {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create trigger dispatcher runtime");
        Self { runtime }
    }

    /// Dispatch trigger evaluation results to the execution pipeline.
    /// Blocking — call from a non-async thread (observer, timer).
    pub fn dispatch(&self, results: &[TriggerEvaluationResult], workflows: &[Workflow]) {
        for result in results {
            let target = ExecutionTarget::workflow(result.workflow_id.clone());
            let wfs = workflows.to_vec();
            log::info!(
                "Dispatching trigger '{}' -> workflow '{}'",
                result.trigger_id,
                result.workflow_id
            );
            let _ = self.runtime.block_on(async move {
                execution::execute_target(&target, "", serde_json::Value::Null, &wfs).await
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{TriggerId, TriggerType, TriggerVersion, WorkflowId, WorkflowVersion};
    use crate::trigger_execution::{TriggerEvaluationResult, evaluate_context_change};

    fn make_trigger(id: &str, wf: &str) -> crate::model::Trigger {
        crate::model::Trigger {
            id: TriggerId::from_string(id),
            name: format!("Trigger {}", id),
            version: TriggerVersion::V1,
            trigger_type: TriggerType::Manual,
            workflow_id: WorkflowId::from_string(wf),
            enabled: true,
        }
    }

    fn make_workflow(id: &str) -> Workflow {
        Workflow {
            id: WorkflowId::from_string(id),
            name: format!("Workflow {}", id),
            version: WorkflowVersion::V1,
            steps: vec![crate::model::WorkflowStep {
                action_id: crate::model::ActionId::from_string("lock"),
                payload: serde_json::json!(null),
            }],
            enabled: true,
        }
    }

    #[test]
    fn dispatcher_handles_empty_results() {
        let dispatcher = TriggerDispatcher::new();
        let workflows = vec![make_workflow("wf-1")];
        dispatcher.dispatch(&[], &workflows);
    }

    #[test]
    fn dispatcher_handles_results_with_no_matching_workflow() {
        let dispatcher = TriggerDispatcher::new();
        let results = vec![TriggerEvaluationResult {
            trigger_id: "t1".into(),
            trigger_type: TriggerType::Manual,
            workflow_id: WorkflowId::from_string("wf-nonexistent"),
        }];
        let workflows = vec![make_workflow("wf-1")];
        dispatcher.dispatch(&results, &workflows);
    }

    #[test]
    fn evaluate_context_change_dispatch_integration() {
        let triggers = vec![
            crate::model::Trigger {
                id: TriggerId::from_string("t1"),
                name: "Trigger t1".into(),
                version: TriggerVersion::V1,
                trigger_type: TriggerType::DesktopStartup,
                workflow_id: WorkflowId::from_string("wf-1"),
                enabled: true,
            },
            crate::model::Trigger {
                id: TriggerId::from_string("t2"),
                name: "Trigger t2".into(),
                version: TriggerVersion::V1,
                trigger_type: TriggerType::ProcessLaunch {
                    process_name: "code.exe".into(),
                },
                workflow_id: WorkflowId::from_string("wf-2"),
                enabled: true,
            },
        ];
        let results = evaluate_context_change("code.exe", None, &triggers);
        assert_eq!(results.len(), 2);
    }
}
