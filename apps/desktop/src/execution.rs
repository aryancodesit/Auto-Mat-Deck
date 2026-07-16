use std::time::Duration;

use crate::actions::ExecutionOutcome;
use crate::agent::ACTIONS;

const EXECUTION_TIMEOUT: Duration = Duration::from_secs(5);

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
    use serde_json::json;

    #[tokio::test]
    async fn execute_action_with_valid_action_returns_success() {
        // "lock" action exists in ACTIONS and returns Ok on all platforms
        // (on non-Windows it skips the OS call and returns Ok)
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
        // "launch" requires "app" field in payload
        let result = execute_action("launch".into(), json!({})).await;
        assert!(matches!(result, ExecutionOutcome::Failed(_)));
    }
}
