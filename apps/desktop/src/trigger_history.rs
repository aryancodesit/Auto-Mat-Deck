use std::collections::VecDeque;
use std::path::Path;

use log::{info, warn};

use crate::model::{TriggerExecutionRecord, TriggerExecutionStatus, TriggerId, WorkflowId};

const DEFAULT_MAX_HISTORY: usize = 100;

/// Bounded ring buffer for trigger execution records.
/// Evicts oldest entries when capacity is reached.
pub struct TriggerHistory {
    records: VecDeque<TriggerExecutionRecord>,
    max_size: usize,
}

impl TriggerHistory {
    pub fn new(max_size: usize) -> Self {
        Self {
            records: VecDeque::with_capacity(max_size.min(1024)),
            max_size,
        }
    }

    pub fn record(
        &mut self,
        trigger_id: TriggerId,
        workflow_id: WorkflowId,
        status: TriggerExecutionStatus,
        timestamp: u64,
        duration_ms: u64,
    ) {
        let record = TriggerExecutionRecord {
            trigger_id,
            workflow_id,
            status,
            timestamp,
            duration_ms,
        };
        if self.records.len() >= self.max_size {
            self.records.pop_front();
        }
        self.records.push_back(record);
    }

    pub fn records(&self) -> impl Iterator<Item = &TriggerExecutionRecord> {
        self.records.iter()
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn clear(&mut self) {
        self.records.clear();
    }

    /// Save trigger history to a JSON file.
    pub fn save_to_file(&self, path: &Path) {
        match serde_json::to_string(&self.records) {
            Ok(json) => {
                if let Err(e) = std::fs::write(path, &json) {
                    warn!("Failed to write trigger_history.json: {}", e);
                }
            }
            Err(e) => warn!("Failed to serialize trigger history: {}", e),
        }
    }

    /// Load trigger history from a JSON file. Returns empty history on error.
    pub fn load_from_file(path: &Path, max_size: usize) -> Self {
        if !path.exists() {
            return Self::new(max_size);
        }
        match std::fs::read_to_string(path) {
            Ok(content) => match serde_json::from_str::<Vec<TriggerExecutionRecord>>(&content) {
                Ok(records) => {
                    let mut h = Self::new(max_size);
                    for record in records {
                        if h.records.len() >= h.max_size {
                            h.records.pop_front();
                        }
                        h.records.push_back(record);
                    }
                    info!("Loaded {} trigger history records", h.records.len());
                    h
                }
                Err(e) => {
                    warn!(
                        "Failed to parse trigger_history.json: {}. Starting empty.",
                        e
                    );
                    Self::new(max_size)
                }
            },
            Err(e) => {
                warn!(
                    "Failed to read trigger_history.json: {}. Starting empty.",
                    e
                );
                Self::new(max_size)
            }
        }
    }

    /// Return the most recent N records (newest first).
    pub fn recent(&self, n: usize) -> Vec<&TriggerExecutionRecord> {
        self.records.iter().rev().take(n).collect()
    }

    /// Serialize the most recent N records as a JSON string for WebSocket transport.
    pub fn to_json_recent(&self, n: usize) -> String {
        let records: Vec<&TriggerExecutionRecord> = self.records.iter().rev().take(n).collect();
        serde_json::to_string(&records).unwrap_or_else(|_| "[]".into())
    }
}

impl Default for TriggerHistory {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_HISTORY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_history_is_empty() {
        let h = TriggerHistory::new(10);
        assert!(h.is_empty());
        assert_eq!(h.len(), 0);
    }

    #[test]
    fn record_adds_entry() {
        let mut h = TriggerHistory::new(10);
        h.record(
            TriggerId::from_string("t1"),
            WorkflowId::from_string("wf1"),
            TriggerExecutionStatus::Success,
            1000,
            50,
        );
        assert_eq!(h.len(), 1);
    }

    #[test]
    fn records_preserve_order() {
        let mut h = TriggerHistory::new(10);
        h.record(
            TriggerId::from_string("t1"),
            WorkflowId::from_string("wf1"),
            TriggerExecutionStatus::Success,
            1000,
            50,
        );
        h.record(
            TriggerId::from_string("t2"),
            WorkflowId::from_string("wf2"),
            TriggerExecutionStatus::Success,
            2000,
            75,
        );
        let ids: Vec<_> = h.records().map(|r| r.trigger_id.as_str()).collect();
        assert_eq!(ids, vec!["t1", "t2"]);
    }

    #[test]
    fn eviction_removes_oldest() {
        let mut h = TriggerHistory::new(3);
        for i in 0..5 {
            h.record(
                TriggerId::from_string(&format!("t{}", i)),
                WorkflowId::from_string("wf1"),
                TriggerExecutionStatus::Success,
                1000 + i,
                50,
            );
        }
        assert_eq!(h.len(), 3);
        let ids: Vec<_> = h.records().map(|r| r.trigger_id.as_str()).collect();
        assert_eq!(ids, vec!["t2", "t3", "t4"]);
    }

    #[test]
    fn recent_returns_newest_first() {
        let mut h = TriggerHistory::new(10);
        for i in 0..5 {
            h.record(
                TriggerId::from_string(&format!("t{}", i)),
                WorkflowId::from_string("wf1"),
                TriggerExecutionStatus::Success,
                1000 + i,
                50,
            );
        }
        let recent = h.recent(3);
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].trigger_id.as_str(), "t4");
        assert_eq!(recent[1].trigger_id.as_str(), "t3");
        assert_eq!(recent[2].trigger_id.as_str(), "t2");
    }

    #[test]
    fn recent_clamped_to_available() {
        let mut h = TriggerHistory::new(10);
        h.record(
            TriggerId::from_string("t1"),
            WorkflowId::from_string("wf1"),
            TriggerExecutionStatus::Success,
            1000,
            50,
        );
        let recent = h.recent(100);
        assert_eq!(recent.len(), 1);
    }

    #[test]
    fn clear_empties_history() {
        let mut h = TriggerHistory::new(10);
        h.record(
            TriggerId::from_string("t1"),
            WorkflowId::from_string("wf1"),
            TriggerExecutionStatus::Success,
            1000,
            50,
        );
        h.clear();
        assert!(h.is_empty());
    }

    #[test]
    fn default_capacity_is_100() {
        let h = TriggerHistory::default();
        assert_eq!(h.max_size, 100);
    }

    #[test]
    fn failed_status_recorded() {
        let mut h = TriggerHistory::new(10);
        h.record(
            TriggerId::from_string("t1"),
            WorkflowId::from_string("wf1"),
            TriggerExecutionStatus::Failed {
                reason: "timeout".into(),
            },
            1000,
            5000,
        );
        let record = h.records().next().unwrap();
        assert!(matches!(
            record.status,
            TriggerExecutionStatus::Failed { ref reason } if reason == "timeout"
        ));
    }

    #[test]
    fn rejected_status_recorded() {
        let mut h = TriggerHistory::new(10);
        h.record(
            TriggerId::from_string("t1"),
            WorkflowId::from_string("wf1"),
            TriggerExecutionStatus::Rejected {
                reason: "workflow_disabled".into(),
            },
            1000,
            0,
        );
        let record = h.records().next().unwrap();
        assert!(matches!(
            record.status,
            TriggerExecutionStatus::Rejected { ref reason } if reason == "workflow_disabled"
        ));
    }

    #[test]
    fn to_json_recent_returns_newest_first() {
        let mut h = TriggerHistory::new(10);
        for i in 0..3 {
            h.record(
                TriggerId::from_string(&format!("t{}", i)),
                WorkflowId::from_string("wf1"),
                TriggerExecutionStatus::Success,
                1000 + i,
                50,
            );
        }
        let json_str = h.to_json_recent(2);
        let records: Vec<serde_json::Value> = serde_json::from_str(&json_str).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0]["trigger_id"], "t2");
        assert_eq!(records[1]["trigger_id"], "t1");
    }

    #[test]
    fn to_json_recent_empty_history() {
        let h = TriggerHistory::new(10);
        assert_eq!(h.to_json_recent(5), "[]");
    }

    #[test]
    fn to_json_recent_clamped_to_available() {
        let mut h = TriggerHistory::new(10);
        h.record(
            TriggerId::from_string("t1"),
            WorkflowId::from_string("wf1"),
            TriggerExecutionStatus::Success,
            1000,
            50,
        );
        let records: Vec<serde_json::Value> = serde_json::from_str(&h.to_json_recent(100)).unwrap();
        assert_eq!(records.len(), 1);
    }

    #[test]
    fn save_and_load_round_trip() {
        let mut h = TriggerHistory::new(10);
        h.record(
            TriggerId::from_string("t1"),
            WorkflowId::from_string("wf1"),
            TriggerExecutionStatus::Success,
            1689600000,
            150,
        );
        h.record(
            TriggerId::from_string("t2"),
            WorkflowId::from_string("wf2"),
            TriggerExecutionStatus::Failed {
                reason: "timeout".into(),
            },
            1689600100,
            5000,
        );

        let dir = std::env::temp_dir().join("amd_test_history");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test_history.json");
        h.save_to_file(&path);

        let loaded = TriggerHistory::load_from_file(&path, 10);
        assert_eq!(loaded.len(), 2);
        let records: Vec<_> = loaded.records().collect();
        assert_eq!(records[0].trigger_id.as_str(), "t1");
        assert_eq!(records[0].timestamp, 1689600000);
        assert_eq!(records[1].trigger_id.as_str(), "t2");
        assert!(matches!(
            records[1].status,
            TriggerExecutionStatus::Failed { ref reason } if reason == "timeout"
        ));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_from_missing_file_returns_empty() {
        let path = std::env::temp_dir().join("amd_test_nonexistent_history.json");
        let h = TriggerHistory::load_from_file(&path, 10);
        assert!(h.is_empty());
    }

    #[test]
    fn load_clamps_to_max_size() {
        let dir = std::env::temp_dir().join("amd_test_history_clamp");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test_history.json");

        // Write 5 records to file
        let mut h = TriggerHistory::new(100);
        for i in 0..5 {
            h.record(
                TriggerId::from_string(&format!("t{}", i)),
                WorkflowId::from_string("wf1"),
                TriggerExecutionStatus::Success,
                1000 + i,
                50,
            );
        }
        h.save_to_file(&path);

        // Load with max_size=3
        let loaded = TriggerHistory::load_from_file(&path, 3);
        assert_eq!(loaded.len(), 3);
        // Should keep last 3 (oldest evicted)
        let ids: Vec<_> = loaded.records().map(|r| r.trigger_id.as_str()).collect();
        assert_eq!(ids, vec!["t2", "t3", "t4"]);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn timestamp_and_duration_preserved() {
        let mut h = TriggerHistory::new(10);
        h.record(
            TriggerId::from_string("t1"),
            WorkflowId::from_string("wf1"),
            TriggerExecutionStatus::Success,
            1689600000,
            150,
        );
        let record = h.records().next().unwrap();
        assert_eq!(record.timestamp, 1689600000);
        assert_eq!(record.duration_ms, 150);
    }
}
