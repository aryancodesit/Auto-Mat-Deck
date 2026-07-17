use std::sync::{Arc, Mutex};

use crate::execution;
use crate::model::{ExecutionTarget, TriggerExecutionStatus, TriggerId, Workflow};
use crate::trigger_execution::TriggerEvaluationResult;
use crate::trigger_history::TriggerHistory;

/// Bridges sync observer/timer threads to async execute_target() pipeline.
/// Creates its own tokio runtime — call from non-async context only.
pub struct TriggerDispatcher {
    runtime: tokio::runtime::Runtime,
    history: Arc<Mutex<TriggerHistory>>,
    history_tx: tokio::sync::watch::Sender<Option<String>>,
}

impl TriggerDispatcher {
    pub fn new(
        history: Arc<Mutex<TriggerHistory>>,
        history_tx: tokio::sync::watch::Sender<Option<String>>,
    ) -> Self {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create trigger dispatcher runtime");
        Self {
            runtime,
            history,
            history_tx,
        }
    }

    /// Dispatch trigger evaluation results to the execution pipeline.
    /// Blocking — call from a non-async thread (observer, timer).
    /// Records each dispatch to trigger history and publishes updates.
    pub fn dispatch(&self, results: &[TriggerEvaluationResult], workflows: &[Workflow]) {
        for result in results {
            let target = ExecutionTarget::workflow(result.workflow_id.clone());
            let wfs = workflows.to_vec();
            let trigger_id = TriggerId::from_string(result.trigger_id.clone());
            let workflow_id = result.workflow_id.clone();
            log::info!(
                "Dispatching trigger '{}' -> workflow '{}'",
                result.trigger_id,
                result.workflow_id
            );
            let start = std::time::Instant::now();
            let outcome = self.runtime.block_on(async move {
                execution::execute_target(&target, "", serde_json::Value::Null, &wfs).await
            });
            let duration_ms = start.elapsed().as_millis() as u64;
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let status = if outcome.accepted {
                match outcome.executed {
                    Some(true) => TriggerExecutionStatus::Success,
                    Some(false) => TriggerExecutionStatus::Failed {
                        reason: outcome.execution_error.unwrap_or_else(|| "unknown".into()),
                    },
                    None => TriggerExecutionStatus::Success,
                }
            } else {
                TriggerExecutionStatus::Rejected {
                    reason: outcome.reason.unwrap_or_else(|| "unknown".into()),
                }
            };
            if let Ok(mut h) = self.history.lock() {
                h.record(trigger_id, workflow_id, status, timestamp, duration_ms);
            }
        }
        if !results.is_empty() {
            if let Ok(h) = self.history.lock() {
                let json = h.to_json_recent(50);
                let _ = self.history_tx.send(Some(json));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{TriggerId, TriggerType, TriggerVersion, WorkflowId, WorkflowVersion};
    use crate::trigger_execution::{TriggerEvaluationResult, evaluate_context_change};

    fn make_history() -> Arc<Mutex<TriggerHistory>> {
        Arc::new(Mutex::new(TriggerHistory::new(100)))
    }

    fn make_history_tx() -> tokio::sync::watch::Sender<Option<String>> {
        let (tx, _) = tokio::sync::watch::channel(None);
        tx
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
        let dispatcher = TriggerDispatcher::new(make_history(), make_history_tx());
        let workflows = vec![make_workflow("wf-1")];
        dispatcher.dispatch(&[], &workflows);
    }

    #[test]
    fn dispatcher_handles_results_with_no_matching_workflow() {
        let history = make_history();
        let dispatcher = TriggerDispatcher::new(history.clone(), make_history_tx());
        let results = vec![TriggerEvaluationResult {
            trigger_id: "t1".into(),
            trigger_type: TriggerType::Manual,
            workflow_id: WorkflowId::from_string("wf-nonexistent"),
        }];
        let workflows = vec![make_workflow("wf-1")];
        dispatcher.dispatch(&results, &workflows);
        let h = history.lock().unwrap();
        assert_eq!(h.len(), 1);
        let record = h.records().next().unwrap();
        assert!(matches!(
            record.status,
            TriggerExecutionStatus::Rejected { .. }
        ));
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

    #[test]
    fn dispatch_records_timestamp_and_duration() {
        let history = make_history();
        let dispatcher = TriggerDispatcher::new(history.clone(), make_history_tx());
        let results = vec![TriggerEvaluationResult {
            trigger_id: "t1".into(),
            trigger_type: TriggerType::Manual,
            workflow_id: WorkflowId::from_string("wf-nonexistent"),
        }];
        let workflows = vec![];
        dispatcher.dispatch(&results, &workflows);
        let h = history.lock().unwrap();
        let record = h.records().next().unwrap();
        assert!(record.timestamp > 0);
        assert!(record.duration_ms < 1000);
    }

    #[test]
    fn multiple_dispatches_recorded_in_order() {
        let history = make_history();
        let dispatcher = TriggerDispatcher::new(history.clone(), make_history_tx());
        let results = vec![
            TriggerEvaluationResult {
                trigger_id: "t1".into(),
                trigger_type: TriggerType::Manual,
                workflow_id: WorkflowId::from_string("wf-nonexistent"),
            },
            TriggerEvaluationResult {
                trigger_id: "t2".into(),
                trigger_type: TriggerType::Manual,
                workflow_id: WorkflowId::from_string("wf-nonexistent"),
            },
        ];
        dispatcher.dispatch(&results, &[]);
        let h = history.lock().unwrap();
        assert_eq!(h.len(), 2);
        let ids: Vec<_> = h.records().map(|r| r.trigger_id.as_str()).collect();
        assert_eq!(ids, vec!["t1", "t2"]);
    }

    #[test]
    fn dispatch_publishes_history_to_watch_channel() {
        let history = make_history();
        let (tx, rx) = tokio::sync::watch::channel(None);
        let dispatcher = TriggerDispatcher::new(history, tx);
        let results = vec![TriggerEvaluationResult {
            trigger_id: "t1".into(),
            trigger_type: TriggerType::Manual,
            workflow_id: WorkflowId::from_string("wf-nonexistent"),
        }];
        dispatcher.dispatch(&results, &[]);
        let snapshot = rx.borrow().clone();
        assert!(snapshot.is_some());
        let records: Vec<serde_json::Value> = serde_json::from_str(&snapshot.unwrap()).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0]["trigger_id"], "t1");
    }

    #[test]
    fn dispatch_empty_results_does_not_publish() {
        let (tx, rx) = tokio::sync::watch::channel(None);
        let dispatcher = TriggerDispatcher::new(make_history(), tx);
        dispatcher.dispatch(&[], &[]);
        assert!(rx.borrow().is_none());
    }
}
