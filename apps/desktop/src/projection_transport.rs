use std::sync::Arc;

use crate::projection::{ProjectionPublisher, RuntimeProjection};

use serde::Serialize;
use tokio::sync::watch;

const SCHEMA_VERSION: u32 = 1;

#[derive(Serialize)]
struct ActiveProfileStateMessage {
    #[serde(rename = "type")]
    msg_type: &'static str,
    schema_version: u32,
    active_profile_id: Option<String>,
}

impl From<&RuntimeProjection> for ActiveProfileStateMessage {
    fn from(p: &RuntimeProjection) -> Self {
        Self {
            msg_type: "active_profile_state",
            schema_version: SCHEMA_VERSION,
            active_profile_id: p.active_profile_id.clone(),
        }
    }
}

pub(crate) struct ProjectionTransportPublisher {
    sender: watch::Sender<Option<Arc<str>>>,
}

impl ProjectionTransportPublisher {
    pub(crate) fn new() -> (Self, watch::Receiver<Option<Arc<str>>>) {
        let (tx, rx) = watch::channel(None);
        (Self { sender: tx }, rx)
    }
}

impl ProjectionPublisher for ProjectionTransportPublisher {
    fn publish(&self, projection: &RuntimeProjection) {
        let msg = ActiveProfileStateMessage::from(projection);
        match serde_json::to_string(&msg) {
            Ok(json) => {
                let _ = self.sender.send_replace(Some(Arc::from(json.as_str())));
            }
            Err(e) => {
                log::warn!("Projection serialization failed: {e}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ProfileId, RuntimeTransition};
    use crate::projection::project;
    use serde_json::Value;

    fn transition(
        context_changed: bool,
        prev: Option<&str>,
        active: Option<&str>,
    ) -> RuntimeTransition {
        RuntimeTransition {
            context_changed,
            previous_profile_id: prev.map(ProfileId::from_string),
            active_profile_id: active.map(ProfileId::from_string),
        }
    }

    // ── DTO Mapping ──

    #[test]
    fn active_profile_id_maps_from_runtime_projection() {
        let proj = project(&transition(true, Some("prev"), Some("active123")));
        let dto = ActiveProfileStateMessage::from(&proj);
        assert_eq!(dto.active_profile_id, Some("active123".into()));
    }

    #[test]
    fn msg_type_is_constant() {
        let proj = project(&transition(true, None, None));
        let dto = ActiveProfileStateMessage::from(&proj);
        assert_eq!(dto.msg_type, "active_profile_state");
    }

    #[test]
    fn schema_version_is_one() {
        let proj = project(&transition(true, None, None));
        let dto = ActiveProfileStateMessage::from(&proj);
        assert_eq!(dto.schema_version, 1);
    }

    #[test]
    fn null_active_profile_id_in_dto() {
        let proj = project(&transition(true, None, None));
        let dto = ActiveProfileStateMessage::from(&proj);
        assert_eq!(dto.active_profile_id, None);
    }

    #[test]
    fn non_null_active_profile_id_in_dto() {
        let proj = project(&transition(true, None, Some("abc")));
        let dto = ActiveProfileStateMessage::from(&proj);
        assert_eq!(dto.active_profile_id, Some("abc".into()));
    }

    // ── Serialization ──

    #[test]
    fn serializes_to_valid_json() {
        let proj = project(&transition(true, None, Some("abc")));
        let dto = ActiveProfileStateMessage::from(&proj);
        let result = serde_json::to_string(&dto);
        assert!(result.is_ok());
    }

    #[test]
    fn serialized_dto_matches_v1_wire_structure() {
        let proj = project(&transition(true, None, Some("pid-1")));
        let dto = ActiveProfileStateMessage::from(&proj);
        let json = serde_json::to_string(&dto).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();
        let obj = parsed.as_object().unwrap();

        assert_eq!(obj.len(), 3);
        assert!(obj.contains_key("type"));
        assert!(obj.contains_key("schema_version"));
        assert!(obj.contains_key("active_profile_id"));
        assert_eq!(obj["type"], "active_profile_state");
        assert_eq!(obj["schema_version"], 1);
        assert_eq!(obj["active_profile_id"], "pid-1");
    }

    #[test]
    fn null_active_profile_serializes_as_json_null() {
        let proj = project(&transition(true, None, None));
        let dto = ActiveProfileStateMessage::from(&proj);
        let json = serde_json::to_string(&dto).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["active_profile_id"], Value::Null);
    }

    #[test]
    fn serialized_json_has_exactly_type_schema_and_active_id_for_none() {
        let proj = project(&transition(true, None, None));
        let dto = ActiveProfileStateMessage::from(&proj);
        let json = serde_json::to_string(&dto).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();
        let obj = parsed.as_object().unwrap();
        assert_eq!(obj.len(), 3);
        assert_eq!(obj["type"], "active_profile_state");
        assert_eq!(obj["schema_version"], 1);
        assert_eq!(obj["active_profile_id"], Value::Null);
    }

    // ── Watch Channel Handoff ──

    #[test]
    fn initial_watch_state_is_none() {
        let (_publisher, rx) = ProjectionTransportPublisher::new();
        assert_eq!(*rx.borrow(), None);
    }

    #[test]
    fn publish_updates_watch_value() {
        let (publisher, mut rx) = ProjectionTransportPublisher::new();
        let proj = project(&transition(true, None, Some("p1")));
        publisher.publish(&proj);

        let current: Option<Arc<str>> = (*rx.borrow_and_update()).clone();
        assert!(current.is_some());
        let parsed: Value = serde_json::from_str(&current.unwrap()).unwrap();
        assert_eq!(parsed["active_profile_id"], "p1");
    }

    #[test]
    fn multiple_publishes_coalesce_to_latest() {
        let (publisher, mut rx) = ProjectionTransportPublisher::new();
        publisher.publish(&project(&transition(true, None, Some("p1"))));
        publisher.publish(&project(&transition(true, None, Some("p2"))));
        publisher.publish(&project(&transition(true, None, Some("p3"))));

        let current: Option<Arc<str>> = (*rx.borrow_and_update()).clone();
        let parsed: Value = serde_json::from_str(&current.unwrap()).unwrap();
        assert_eq!(parsed["active_profile_id"], "p3");
    }

    #[test]
    fn new_receiver_sees_latest_value() {
        let (publisher, _rx) = ProjectionTransportPublisher::new();
        publisher.publish(&project(&transition(true, None, Some("p1"))));
        publisher.publish(&project(&transition(true, None, Some("p2"))));

        let late_rx = publisher.sender.subscribe();
        let current: Option<Arc<str>> = (*late_rx.borrow()).clone();
        let parsed: Value = serde_json::from_str(&current.unwrap()).unwrap();
        assert_eq!(parsed["active_profile_id"], "p2");
    }

    #[test]
    fn send_replace_succeeds_with_zero_receivers() {
        let rx = {
            let (publisher, rx) = ProjectionTransportPublisher::new();
            publisher.publish(&project(&transition(true, None, Some("p1"))));
            rx
        };
        // rx is dropped — no receivers
        // publisher was also dropped in this scope, so this test only proves
        // that send_replace doesn't panic when the sender exists with no receivers.
        // For a true zero-receiver test while sender is alive, we do it inline:
        let (publisher, rx) = ProjectionTransportPublisher::new();
        drop(rx);
        // No receivers exist
        publisher.publish(&project(&transition(true, None, Some("p1"))));
        // If we reach here, send_replace succeeded
        let new_rx = publisher.sender.subscribe();
        let current: Option<Arc<str>> = (*new_rx.borrow()).clone();
        assert!(current.is_some());
    }

    #[test]
    fn sender_drop_is_observable_by_receiver() {
        let rx = {
            let (publisher, rx) = ProjectionTransportPublisher::new();
            publisher.publish(&project(&transition(true, None, Some("p1"))));
            rx
        };
        // publisher dropped — sender dropped
        assert!(rx.has_changed().is_err());
    }
}
