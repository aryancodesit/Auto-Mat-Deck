use std::sync::Arc;

use crate::projection::{
    ControlSurfaceState, DerivationResult, ProjectionPublisher, RuntimeProjection,
};

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

// ── Control Surface Transport ──

#[derive(Serialize)]
struct ControlSurfaceStateMessage {
    #[serde(rename = "type")]
    msg_type: &'static str,
    schema_version: u32,
    profile_id: Option<String>,
    profile_name: Option<String>,
    pages: Option<Vec<PageProjectionMessage>>,
}

#[derive(Serialize)]
struct PageProjectionMessage {
    page_id: String,
    name: String,
    buttons: Vec<ButtonProjectionMessage>,
}

#[derive(Serialize)]
struct ButtonProjectionMessage {
    button_id: String,
    label: String,
}

impl From<&ControlSurfaceState> for ControlSurfaceStateMessage {
    fn from(s: &ControlSurfaceState) -> Self {
        Self {
            msg_type: "control_surface_state",
            schema_version: SCHEMA_VERSION,
            profile_id: s.profile_id.clone(),
            profile_name: s.profile_name.clone(),
            pages: s.pages.as_ref().map(|pages| {
                pages
                    .iter()
                    .map(|p| PageProjectionMessage {
                        page_id: p.page_id.clone(),
                        name: p.name.clone(),
                        buttons: p
                            .buttons
                            .iter()
                            .map(|b| ButtonProjectionMessage {
                                button_id: b.button_id.clone(),
                                label: b.label.clone(),
                            })
                            .collect(),
                    })
                    .collect()
            }),
        }
    }
}

pub(crate) struct ControlSurfaceTransportPublisher {
    sender: watch::Sender<Option<Arc<str>>>,
}

impl ControlSurfaceTransportPublisher {
    pub(crate) fn new() -> (Self, watch::Receiver<Option<Arc<str>>>) {
        let (tx, rx) = watch::channel(None);
        (Self { sender: tx }, rx)
    }

    pub(crate) fn publish_derivation(&self, result: &DerivationResult) {
        match result {
            DerivationResult::Failed => {}
            DerivationResult::Published(state) => {
                let msg = ControlSurfaceStateMessage::from(state);
                match serde_json::to_string(&msg) {
                    Ok(json) => {
                        let _ = self.sender.send_replace(Some(Arc::from(json.as_str())));
                    }
                    Err(e) => {
                        log::warn!("Control surface serialization failed: {e}");
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ProfileId, RuntimeTransition};
    use crate::projection::{
        ButtonProjection, ControlSurfaceState, DerivationResult, PageProjection, project,
    };
    use serde_json::Value;

    // ── APS DTO tests (existing) ──

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
        let _outer_rx = {
            let (publisher, rx) = ProjectionTransportPublisher::new();
            publisher.publish(&project(&transition(true, None, Some("p1"))));
            rx
        };
        let (publisher, rx) = ProjectionTransportPublisher::new();
        drop(rx);
        publisher.publish(&project(&transition(true, None, Some("p1"))));
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
        assert!(rx.has_changed().is_err());
    }

    // ── Control Surface Transport Tests ──

    #[test]
    fn css_msg_type_is_constant() {
        let state = ControlSurfaceState {
            profile_id: None,
            profile_name: None,
            pages: None,
        };
        let dto = ControlSurfaceStateMessage::from(&state);
        assert_eq!(dto.msg_type, "control_surface_state");
    }

    #[test]
    fn css_schema_version_is_one() {
        let state = ControlSurfaceState {
            profile_id: None,
            profile_name: None,
            pages: None,
        };
        let dto = ControlSurfaceStateMessage::from(&state);
        assert_eq!(dto.schema_version, 1);
    }

    #[test]
    fn css_null_triple_serializes_correctly() {
        let cs = ControlSurfaceState {
            profile_id: None,
            profile_name: None,
            pages: None,
        };
        let dto = ControlSurfaceStateMessage::from(&cs);
        let json = serde_json::to_string(&dto).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();
        let obj = parsed.as_object().unwrap();
        assert_eq!(obj["type"], "control_surface_state");
        assert_eq!(obj["schema_version"], 1);
        assert!(
            obj.contains_key("profile_id"),
            "profile_id key must be present (not skipped)"
        );
        assert!(
            obj.contains_key("profile_name"),
            "profile_name key must be present (not skipped)"
        );
        assert!(
            obj.contains_key("pages"),
            "pages key must be present (not skipped)"
        );
        assert_eq!(obj["profile_id"], Value::Null);
        assert_eq!(obj["profile_name"], Value::Null);
        assert_eq!(obj["pages"], Value::Null);
    }

    #[test]
    fn css_active_profile_serializes_correctly() {
        let cs = ControlSurfaceState {
            profile_id: Some("p1".into()),
            profile_name: Some("Coding".into()),
            pages: Some(vec![PageProjection {
                page_id: "pg1".into(),
                name: "Main".into(),
                buttons: vec![ButtonProjection {
                    button_id: "b1".into(),
                    label: "Compile".into(),
                }],
            }]),
        };
        let dto = ControlSurfaceStateMessage::from(&cs);
        let json = serde_json::to_string(&dto).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();
        let obj = parsed.as_object().unwrap();
        assert_eq!(obj.len(), 5, "CSS object must have exactly 5 fields");
        assert_eq!(obj["profile_id"], "p1");
        assert_eq!(obj["profile_name"], "Coding");
        assert_eq!(obj["pages"][0]["page_id"], "pg1");
        assert_eq!(obj["pages"][0]["name"], "Main");
        assert_eq!(obj["pages"][0]["buttons"][0]["button_id"], "b1");
        assert_eq!(obj["pages"][0]["buttons"][0]["label"], "Compile");
    }

    #[test]
    fn css_zero_pages_serializes_as_empty_array() {
        let cs = ControlSurfaceState {
            profile_id: Some("p1".into()),
            profile_name: Some("Empty".into()),
            pages: Some(vec![]),
        };
        let dto = ControlSurfaceStateMessage::from(&cs);
        let json = serde_json::to_string(&dto).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["pages"], Value::Array(vec![]));
    }

    #[test]
    fn css_excludes_action_reference() {
        let cs = ControlSurfaceState {
            profile_id: Some("p1".into()),
            profile_name: Some("Test".into()),
            pages: Some(vec![PageProjection {
                page_id: "pg1".into(),
                name: "Main".into(),
                buttons: vec![ButtonProjection {
                    button_id: "b1".into(),
                    label: "Do it".into(),
                }],
            }]),
        };
        let dto = ControlSurfaceStateMessage::from(&cs);
        let json = serde_json::to_string(&dto).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();
        // Verify only allowed fields exist at button level
        let btn = &parsed["pages"][0]["buttons"][0];
        let btn_obj = btn.as_object().unwrap();
        assert_eq!(btn_obj.len(), 2, "button should have exactly 2 fields");
        assert!(btn_obj.contains_key("button_id"));
        assert!(btn_obj.contains_key("label"));
    }

    #[test]
    fn css_publisher_initial_state_is_none() {
        let (_pub, rx) = ControlSurfaceTransportPublisher::new();
        assert_eq!(*rx.borrow(), None);
    }

    #[test]
    fn css_publish_updates_watch_value() {
        let (publisher, mut rx) = ControlSurfaceTransportPublisher::new();
        let result = DerivationResult::Published(ControlSurfaceState {
            profile_id: Some("p1".into()),
            profile_name: Some("P1".into()),
            pages: Some(vec![]),
        });
        publisher.publish_derivation(&result);
        let current: Option<Arc<str>> = (*rx.borrow_and_update()).clone();
        assert!(current.is_some());
        let parsed: Value = serde_json::from_str(&current.unwrap()).unwrap();
        assert_eq!(parsed["profile_id"], "p1");
    }

    #[test]
    fn css_publish_failure_does_not_update_watch() {
        let (publisher, rx) = ControlSurfaceTransportPublisher::new();
        assert_eq!(*rx.borrow(), None);
        publisher.publish_derivation(&DerivationResult::Failed);
        assert_eq!(*rx.borrow(), None);
    }

    #[test]
    fn css_multiple_publishes_coalesce_to_latest() {
        let (publisher, mut rx) = ControlSurfaceTransportPublisher::new();
        publisher.publish_derivation(&DerivationResult::Published(ControlSurfaceState {
            profile_id: Some("p1".into()),
            profile_name: Some("P1".into()),
            pages: Some(vec![]),
        }));
        publisher.publish_derivation(&DerivationResult::Published(ControlSurfaceState {
            profile_id: Some("p2".into()),
            profile_name: Some("P2".into()),
            pages: Some(vec![]),
        }));
        let current: Option<Arc<str>> = (*rx.borrow_and_update()).clone();
        let parsed: Value = serde_json::from_str(&current.unwrap()).unwrap();
        assert_eq!(parsed["profile_id"], "p2");
    }

    #[test]
    fn css_new_receiver_sees_latest() {
        let (publisher, _rx) = ControlSurfaceTransportPublisher::new();
        publisher.publish_derivation(&DerivationResult::Published(ControlSurfaceState {
            profile_id: Some("p1".into()),
            profile_name: Some("P1".into()),
            pages: Some(vec![]),
        }));
        publisher.publish_derivation(&DerivationResult::Published(ControlSurfaceState {
            profile_id: Some("p2".into()),
            profile_name: Some("P2".into()),
            pages: Some(vec![]),
        }));
        let late = publisher.sender.subscribe();
        let current: Option<Arc<str>> = (*late.borrow()).clone();
        let parsed: Value = serde_json::from_str(&current.unwrap()).unwrap();
        assert_eq!(parsed["profile_id"], "p2");
    }

    #[test]
    fn css_sender_dropped_receiver_observes() {
        let rx = {
            let (publisher, rx) = ControlSurfaceTransportPublisher::new();
            publisher.publish_derivation(&DerivationResult::Published(ControlSurfaceState {
                profile_id: Some("p1".into()),
                profile_name: Some("P1".into()),
                pages: Some(vec![]),
            }));
            rx
        };
        assert!(rx.has_changed().is_err());
    }
}
