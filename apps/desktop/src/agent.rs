use std::net::SocketAddr;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;

use crate::actions::ActionRegistry;
use crate::pairing::{SharedPairingManager, ValidationResult, validation_reason_code};
use crate::repository::DocumentStore;
use crate::projection::{RejectionReason, validate_button};
use crate::state::SharedRuntime;

use futures_util::{SinkExt, StreamExt};
use log::{error, info, warn};
use serde_json::{Value, json};
use tokio::net::TcpListener;
use tokio::sync::watch;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

pub const PORT: u16 = 9742;
const PAIR_TIMEOUT_SECS: u64 = 120;

pub static ACTIONS: LazyLock<ActionRegistry> = LazyLock::new(|| ActionRegistry::new());

pub struct PendingPair {
    pub device_id: String,
    pub device_name: String,
    pub responder: tokio::sync::oneshot::Sender<bool>,
}

pub type PairState = Arc<Mutex<Option<PendingPair>>>;

pub async fn run_server(
    mut shutdown_rx: watch::Receiver<bool>,
    pair_state: PairState,
    pairing_manager: SharedPairingManager,
    shared: SharedRuntime,
    store: Arc<dyn DocumentStore>,
    projection_state_rx: watch::Receiver<Option<Arc<str>>>,
    control_surface_state_rx: watch::Receiver<Option<Arc<str>>>,
) {
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    let device_id = format!("amd-{}", &hostname);

    // Bind WebSocket BEFORE starting mDNS — don't advertise a port we aren't listening on.
    let addr: SocketAddr = ([0, 0, 0, 0], PORT).into();
    info!("Binding WebSocket server to {}...", addr);
    let listener = match TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            error!("FATAL: Failed to bind WebSocket server on {}: {}", addr, e);
            return;
        }
    };
    info!("WebSocket server listening on ws://{}", addr);

    // Now that the server is confirmed ready, start advertising.
    let mut advertisers: Vec<Box<dyn crate::discovery::AdvertisementProvider>> = Vec::new();
    advertisers.push(Box::new(crate::discovery::MdnsAnnouncer::new(
        device_id.clone(),
        hostname.clone(),
        PORT,
    )));

    for advertiser in advertisers.iter_mut() {
        match advertiser.start() {
            Ok(()) => info!(
                "[{}] Started: device_id={}",
                advertiser.provider_name(),
                advertiser.device_id()
            ),
            Err(e) => warn!("[{}] Failed to start: {}", advertiser.provider_name(), e),
        }
    }

    info!(
        "Desktop agent ready. Hostname: {}, Device ID: {}, Port: {}",
        hostname, device_id, PORT
    );

    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, peer)) => {
                            info!("[CONNECT] Incoming TCP connection from {}", peer);
                            let ss = shared.clone();
                            let st = store.clone();
                            let pm = pairing_manager.clone();
                            let prx = projection_state_rx.clone();
                            let crx = control_surface_state_rx.clone();
                            tokio::spawn(handle_connection(stream, peer, pair_state.clone(), pm, ss, st, prx, crx));
                    }
                    Err(e) => {
                        error!("[CONNECT] Accept error: {}", e);
                        break;
                    }
                }
            }
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    info!("Shutdown signal received, stopping server...");
                    break;
                }
            }
        }
    }
}

fn is_trusted(shared: &SharedRuntime, device_id: &str) -> bool {
    shared.lock().unwrap().app.is_trusted(device_id)
}

fn touch_device(shared: &SharedRuntime, store: &dyn DocumentStore, device_id: &str) {
    let mut rt = shared.lock().unwrap();
    rt.app.touch_device(device_id);
    rt.app.persist(store);
}

fn add_device(
    shared: &SharedRuntime,
    store: &dyn DocumentStore,
    device_id: &str,
    device_name: &str,
) {
    let mut rt = shared.lock().unwrap();
    rt.app.add_device(device_id, device_name);
    rt.app.persist(store);
}

async fn handle_connection(
    stream: tokio::net::TcpStream,
    peer: SocketAddr,
    pair_state: PairState,
    pairing_manager: SharedPairingManager,
    shared: SharedRuntime,
    store: Arc<dyn DocumentStore>,
    mut projection_rx: watch::Receiver<Option<Arc<str>>>,
    mut control_surface_state_rx: watch::Receiver<Option<Arc<str>>>,
) {
    let ws_stream = match accept_async(stream).await {
        Ok(ws) => {
            info!("[WS] WebSocket upgrade successful from {}", peer);
            ws
        }
        Err(e) => {
            warn!("[WS] WebSocket handshake failed from {}: {}", peer, e);
            return;
        }
    };

    let (mut write, mut read) = ws_stream.split();
    let mut device_id: Option<String> = None;
    let mut projection_active = false;
    let mut aps_open = true;
    let mut css_open = true;

    loop {
        // Before projection is active: only inbound messages.
        // After projection is active: multiplex inbound + projection changes.
        let inbound = if projection_active {
            tokio::select! {
                msg = read.next() => msg,
                changed = projection_rx.changed(), if aps_open => {
                    match changed {
                        Ok(()) => {
                            let snapshot = projection_rx.borrow_and_update().clone();
                            if let Some(payload) = snapshot {
                                let text = Message::Text(payload.to_string().into());
                                if let Err(e) = write.send(text).await {
                                    warn!("[PROJ] Write failed for {}: {}", peer, e);
                                    break;
                                }
                            }
                        }
                        Err(_) => {
                            info!("[PROJ] APS channel closed for {}", peer);
                            aps_open = false;
                        }
                    }
                    continue;
                }
                changed = control_surface_state_rx.changed(), if css_open => {
                    match changed {
                        Ok(()) => {
                            let snapshot = control_surface_state_rx.borrow_and_update().clone();
                            if let Some(payload) = snapshot {
                                let text = Message::Text(payload.to_string().into());
                                if let Err(e) = write.send(text).await {
                                    warn!("[CSS] Write failed for {}: {}", peer, e);
                                    break;
                                }
                            }
                        }
                        Err(_) => {
                            warn!("[CSS] Control surface channel closed for {}", peer);
                            css_open = false;
                        }
                    }
                    continue;
                }
            }
        } else {
            read.next().await
        };

        let Some(msg) = inbound else {
            break;
        };

        match msg {
            Ok(Message::Text(text)) => match serde_json::from_str::<Value>(&text) {
                Ok(req) => {
                    let msg_type = req
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");

                    match msg_type {
                        "identify" => {
                            let id = req
                                .get("device_id")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            let name = req
                                .get("device_name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                                .to_string();
                            device_id = Some(id.clone());

                            if is_trusted(&shared, &id) {
                                touch_device(&shared, &*store, &id);
                                info!(
                                    "[PAIR] Trusted device reconnected: {} ({}) from {}",
                                    name, id, peer
                                );
                                let resp = json!({"type": "trusted", "device_id": id});
                                if let Err(e) =
                                    write.send(Message::Text(resp.to_string().into())).await
                                {
                                    warn!("[PAIR] Failed to send trusted to {}: {}", peer, e);
                                    break;
                                }
                                // Trust acknowledgement succeeded — activate projection
                                let snapshot = projection_rx.borrow_and_update().clone();
                                if let Some(payload) = snapshot {
                                    if let Err(e) =
                                        write.send(Message::Text(payload.to_string().into())).await
                                    {
                                        warn!("[PROJ] Write failed for {}: {}", peer, e);
                                        break;
                                    }
                                }
                                // Send retained CSS projection
                                let css_snapshot =
                                    control_surface_state_rx.borrow_and_update().clone();
                                if let Some(payload) = css_snapshot {
                                    if let Err(e) =
                                        write.send(Message::Text(payload.to_string().into())).await
                                    {
                                        warn!("[CSS] Write failed for {}: {}", peer, e);
                                        break;
                                    }
                                }
                                projection_active = true;
                            } else {
                                info!(
                                    "[PAIR] Unknown device identified: {} ({}) from {}",
                                    name, id, peer
                                );
                                let resp = json!({
                                    "type": "untrusted",
                                    "message": "Device not paired. Send pair_request to initiate pairing."
                                });
                                let _ = write.send(Message::Text(resp.to_string().into())).await;
                            }
                        }

                        "pair_request" => {
                            if projection_active {
                                info!("[PAIR] pair_request from already-trusted device, ignoring");
                                let resp = json!({"type": "error", "message": "Already paired"});
                                let _ = write.send(Message::Text(resp.to_string().into())).await;
                                continue;
                            }

                            let id = match device_id {
                                Some(ref id) => id.clone(),
                                None => {
                                    info!(
                                        "[PAIR] pair_request without prior identify from {}",
                                        peer
                                    );
                                    let resp =
                                        json!({"type": "error", "message": "Must identify first"});
                                    let _ =
                                        write.send(Message::Text(resp.to_string().into())).await;
                                    continue;
                                }
                            };

                            let device_name = req
                                .get("device_name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                                .to_string();

                            // Try pairing_code from the request first (v0.2 OTP pairing)
                            let pairing_code = req.get("pairing_code").and_then(|v| v.as_str());

                            if let Some(code) = pairing_code {
                                let result = pairing_manager.validate_code(code);
                                let reason_code = validation_reason_code(&result);
                                match result {
                                    ValidationResult::Accepted => {
                                        add_device(&shared, &*store, &id, &device_name);
                                        info!(
                                            "[PAIR] OTP pair ACCEPTED for {} ({})",
                                            device_name, id
                                        );
                                        let resp =
                                            json!({"type": "pair_accepted", "device_id": id});
                                        if let Err(e) =
                                            write.send(Message::Text(resp.to_string().into())).await
                                        {
                                            warn!(
                                                "[PAIR] Failed to send pair_accepted to {}: {}",
                                                peer, e
                                            );
                                            break;
                                        }
                                        // pair_accepted acknowledgement succeeded — activate projection
                                        let snapshot = projection_rx.borrow_and_update().clone();
                                        if let Some(payload) = snapshot {
                                            if let Err(e) = write
                                                .send(Message::Text(payload.to_string().into()))
                                                .await
                                            {
                                                warn!("[PROJ] Write failed for {}: {}", peer, e);
                                                break;
                                            }
                                        }
                                        // Send retained CSS projection
                                        let css_snapshot =
                                            control_surface_state_rx.borrow_and_update().clone();
                                        if let Some(payload) = css_snapshot {
                                            if let Err(e) = write
                                                .send(Message::Text(payload.to_string().into()))
                                                .await
                                            {
                                                warn!("[CSS] Write failed for {}: {}", peer, e);
                                                break;
                                            }
                                        }
                                        projection_active = true;
                                        continue;
                                    }
                                    _ => {
                                        info!(
                                            "[PAIR] pairing_code rejected (reason={}) from {} ({})",
                                            reason_code, device_name, id
                                        );
                                        let resp = json!({
                                            "type": "pair_rejected",
                                            "device_id": id,
                                            "reason": reason_code
                                        });
                                        let _ = write
                                            .send(Message::Text(resp.to_string().into()))
                                            .await;
                                        continue;
                                    }
                                }
                            }

                            // Fallback: tray approval (legacy v0.1 path)
                            let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();

                            {
                                let mut state = pair_state.lock().unwrap();
                                *state = Some(PendingPair {
                                    device_id: id.clone(),
                                    device_name: device_name.clone(),
                                    responder: resp_tx,
                                });
                            }
                            info!(
                                "[PAIR] Pair request (no OTP) from {} ({}) from {}, awaiting tray approval",
                                device_name, id, peer
                            );

                            let timeout_result = tokio::time::timeout(
                                Duration::from_secs(PAIR_TIMEOUT_SECS),
                                resp_rx,
                            )
                            .await;

                            let (approved, tray_reason) = match timeout_result {
                                Ok(Ok(true)) => (true, ""),
                                Ok(Ok(false)) => (false, "user_declined"),
                                Ok(Err(_)) => (false, "user_declined"),
                                Err(_) => (false, "timeout"),
                            };

                            {
                                let mut state = pair_state.lock().unwrap();
                                *state = None;
                            }

                            if approved {
                                add_device(&shared, &*store, &id, &device_name);
                                info!("[PAIR] Tray ACCEPTED for {} ({})", device_name, id);
                                let resp = json!({"type": "pair_accepted", "device_id": id});
                                if let Err(e) =
                                    write.send(Message::Text(resp.to_string().into())).await
                                {
                                    warn!("[PAIR] Failed to send pair_accepted to {}: {}", peer, e);
                                    break;
                                }
                                // pair_accepted acknowledgement succeeded — activate projection
                                let snapshot = projection_rx.borrow_and_update().clone();
                                if let Some(payload) = snapshot {
                                    if let Err(e) =
                                        write.send(Message::Text(payload.to_string().into())).await
                                    {
                                        warn!("[PROJ] Write failed for {}: {}", peer, e);
                                        break;
                                    }
                                }
                                // Send retained CSS projection
                                let css_snapshot =
                                    control_surface_state_rx.borrow_and_update().clone();
                                if let Some(payload) = css_snapshot {
                                    if let Err(e) =
                                        write.send(Message::Text(payload.to_string().into())).await
                                    {
                                        warn!("[CSS] Write failed for {}: {}", peer, e);
                                        break;
                                    }
                                }
                                projection_active = true;
                            } else {
                                info!(
                                    "[PAIR] Pair REJECTED (reason={}) for {} ({})",
                                    tray_reason, device_name, id
                                );
                                let resp = json!({
                                    "type": "pair_rejected",
                                    "device_id": id,
                                    "reason": tray_reason
                                });
                                if let Err(e) =
                                    write.send(Message::Text(resp.to_string().into())).await
                                {
                                    error!(
                                        "[PAIR] Failed to send pair_rejected to {}: {}",
                                        peer, e
                                    );
                                }
                            }
                        }

                        "ping" => {
                            if !projection_active && device_id.is_some() {
                                let resp = json!({"type": "error", "message": "Device not trusted. Complete pairing first."});
                                let _ = write.send(Message::Text(resp.to_string().into())).await;
                                continue;
                            }
                            let resp = json!({
                                "type": "pong",
                                "echo": req,
                                "deviceId": peer.to_string()
                            });
                            let resp_text = serde_json::to_string(&resp).unwrap();
                            if let Err(e) = write.send(Message::Text(resp_text.into())).await {
                                warn!("Failed to send pong to {}: {}", peer, e);
                                break;
                            }
                        }

                        "action" => {
                            if !projection_active {
                                let rid = req
                                    .get("request_id")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("unknown");
                                let resp = json!({
                                    "type": "error",
                                    "request_id": rid,
                                    "message": "Device not paired. Complete pairing first."
                                });
                                let _ = write.send(Message::Text(resp.to_string().into())).await;
                                continue;
                            }

                            let request_id = req
                                .get("request_id")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                                .to_string();

                            let action_name =
                                req.get("action").and_then(|v| v.as_str()).unwrap_or("");

                            let empty = json!({});
                            let payload = req.get("payload").unwrap_or(&empty);

                            info!(
                                "Action from {}: action={}, request_id={}",
                                peer, action_name, request_id
                            );

                            let result = ACTIONS.execute(action_name, payload);

                            let resp = match result {
                                Ok(data) => json!({
                                    "type": "action_result",
                                    "request_id": request_id,
                                    "success": true,
                                    "data": data
                                }),
                                Err(e) => json!({
                                    "type": "action_result",
                                    "request_id": request_id,
                                    "success": false,
                                    "error": e.message
                                }),
                            };

                            if let Err(e) = write.send(Message::Text(resp.to_string().into())).await
                            {
                                warn!("Failed to send action result to {}: {}", peer, e);
                            break;
                        }
                    }

                    "control_invoke" => {
                        let button_id = req
                            .get("button_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");

                        let (accepted, reason) = if !projection_active {
                            (false, Some("no_active_profile"))
                        } else {
                            let (active_pid, profiles) = {
                                let rt = shared.lock().unwrap();
                                (
                                    rt.runtime.active_profile_id.clone(),
                                    rt.app.document.profiles.clone(),
                                )
                            };
                            // ponytail: clone profiles out, release lock before .await
                            match validate_button(active_pid.as_ref(), &profiles, button_id) {
                                Ok(_) => (true, None),
                                Err(RejectionReason::NoActiveProfile) => {
                                    (false, Some("no_active_profile"))
                                }
                                Err(RejectionReason::UnknownButton) => {
                                    (false, Some("unknown_button"))
                                }
                                Err(RejectionReason::AmbiguousButton) => {
                                    (false, Some("ambiguous_button"))
                                }
                            }
                        };

                        let mut resp = json!({
                            "type": "control_invoke_result",
                            "schema_version": 1,
                            "button_id": button_id,
                            "accepted": accepted,
                        });
                        if let Some(r) = reason {
                            resp["reason"] = json!(r);
                        }

                        info!(
                            "control_invoke from {}: button_id={}, accepted={}",
                            peer, button_id, accepted
                        );

                        if let Err(e) =
                            write.send(Message::Text(resp.to_string().into())).await
                        {
                            warn!(
                                "Failed to send control_invoke_result to {}: {}",
                                peer, e
                            );
                            break;
                        }
                    }

                    _ => {
                        let resp = json!({"type": "error", "message": format!("Unknown message type: {}", msg_type)});
                            let _ = write.send(Message::Text(resp.to_string().into())).await;
                        }
                    }
                }
                Err(e) => {
                    warn!("Invalid JSON from {}: {}", peer, e);
                    let resp = json!({"type": "error", "message": "Invalid JSON"});
                    let _ = write.send(Message::Text(resp.to_string().into())).await;
                }
            },
            Ok(Message::Close(_)) => {
                if let Some(ref id) = device_id {
                    info!("Connection closed by {} ({})", peer, id);
                } else {
                    info!("Connection closed by {}", peer);
                }
                break;
            }
            Ok(Message::Ping(data)) => {
                let _ = write.send(Message::Pong(data)).await;
            }
            Err(e) => {
                warn!("WebSocket error from {}: {}", peer, e);
                break;
            }
            _ => {}
        }
    }

    if let Some(ref id) = device_id {
        info!("Connection closed: {} ({})", peer, id);
    } else {
        info!("Connection closed: {}", peer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pairing::PairingManager;
    use crate::state::DesktopRuntime;
    use futures_util::SinkExt;
    use tokio_tungstenite::connect_async;

    fn make_runtime() -> SharedRuntime {
        Arc::new(Mutex::new(DesktopRuntime::new(
            &crate::repository::JsonRepository::new(),
        )))
    }

    fn add_trusted_device(shared: &SharedRuntime, device_id: &str) {
        let mut rt = shared.lock().unwrap();
        rt.app.add_device(device_id, "test-device");
    }

    fn make_pairing_manager() -> SharedPairingManager {
        Arc::new(PairingManager::new())
    }

    /// Read the next WebSocket text message and return the parsed JSON.
    /// Panics on timeout, error, or non-text message.
    async fn recv_json(
        ws: &mut (
                 impl futures_util::StreamExt<
            Item = Result<Message, tokio_tungstenite::tungstenite::Error>,
        > + Unpin
             ),
        timeout: std::time::Duration,
    ) -> serde_json::Value {
        let msg = tokio::time::timeout(timeout, ws.next())
            .await
            .expect("timeout waiting for message")
            .expect("stream ended")
            .expect("ws error");
        match msg {
            Message::Text(text) => serde_json::from_str(&text).expect("valid JSON"),
            other => panic!("expected Text, got {:?}", other),
        }
    }

    /// Search incoming WebSocket messages for one with the given `type` field.
    /// Consumes messages until found, then returns the full parsed JSON.
    async fn recv_message_type(
        ws: &mut (
                 impl futures_util::StreamExt<
            Item = Result<Message, tokio_tungstenite::tungstenite::Error>,
        > + Unpin
             ),
        expected_type: &str,
        timeout: std::time::Duration,
        max_messages: usize,
    ) -> serde_json::Value {
        for i in 0..max_messages {
            let msg = tokio::time::timeout(timeout, ws.next())
                .await
                .unwrap_or_else(|_| {
                    panic!(
                        "timeout after {} message(s) waiting for type={}",
                        i, expected_type
                    )
                })
                .unwrap_or_else(|| {
                    panic!(
                        "stream ended after {} message(s) waiting for type={}",
                        i, expected_type
                    )
                })
                .unwrap_or_else(|e| {
                    panic!(
                        "ws error after {} message(s) waiting for type={}: {}",
                        i, expected_type, e
                    )
                });
            match msg {
                Message::Text(text) => {
                    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
                    if parsed["type"] == expected_type {
                        return parsed;
                    }
                }
                _ => {}
            }
        }
        panic!(
            "did not find type={} after checking {} messages",
            expected_type, max_messages
        );
    }

    /// Run a WebSocket client that sends `requests` and collects responses,
    /// while the server side runs `handle_connection`.
    async fn run_test(
        shared: SharedRuntime,
        pairing_manager: SharedPairingManager,
        projection_tx: watch::Sender<Option<Arc<str>>>,
        requests: Vec<serde_json::Value>,
        timeout: std::time::Duration,
    ) -> Vec<serde_json::Value> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().unwrap();
        let url = format!("ws://{}/", addr);

        let pair_state: PairState = Arc::new(Mutex::new(None));
        let projection_rx = projection_tx.subscribe();
        let (_css_tx, css_rx) = watch::channel(None);
        let shared_clone = shared.clone();
        let pm = pairing_manager.clone();
        let ps = pair_state.clone();

        // Server accepts one connection
        let server_handle = tokio::spawn(async move {
            let (tcp_stream, peer) = listener.accept().await.expect("accept");
            handle_connection(
                tcp_stream,
                peer,
                ps,
                pm,
                shared_clone,
                Arc::new(crate::repository::JsonRepository::new()),
                projection_rx,
                css_rx,
            )
            .await;
        });

        // Give server a moment to start accepting
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Client connects
        let (mut ws_stream, _) = connect_async(&url).await.expect("connect");

        // Send each request
        for req in &requests {
            let text = serde_json::to_string(req).unwrap();
            ws_stream
                .send(Message::Text(text.into()))
                .await
                .expect("send");
        }

        // Collect responses within timeout
        let mut responses = Vec::new();
        loop {
            match tokio::time::timeout(timeout, ws_stream.next()).await {
                Ok(Some(Ok(Message::Text(text)))) => {
                    let parsed: serde_json::Value =
                        serde_json::from_str(&text).expect("valid JSON response");
                    responses.push(parsed);
                }
                Ok(Some(Ok(Message::Close(_)))) | Ok(None) => break,
                Ok(Some(Err(e))) => {
                    panic!("WS error: {}", e);
                }
                Err(_) => break, // timeout — no more messages expected
                _ => {}
            }
        }

        // Drop client, server will see read error and exit
        drop(ws_stream);
        let _ = tokio::time::timeout(std::time::Duration::from_secs(1), server_handle).await;

        responses
    }

    fn identify(device_id: &str) -> serde_json::Value {
        json!({"type": "identify", "device_id": device_id, "device_name": "test-client"})
    }

    fn pair_request_otp(code: &str) -> serde_json::Value {
        json!({"type": "pair_request", "device_name": "test-client", "pairing_code": code})
    }

    fn make_css_payload(
        profile_id: &str,
        profile_name: &str,
        page_id: &str,
        page_name: &str,
        button_id: &str,
        label: &str,
    ) -> String {
        json!({
            "type": "control_surface_state",
            "schema_version": 1,
            "profile_id": profile_id,
            "profile_name": profile_name,
            "pages": [{
                "page_id": page_id,
                "name": page_name,
                "buttons": [{
                    "button_id": button_id,
                    "label": label
                }]
            }]
        })
        .to_string()
    }

    // ── Tests ──

    #[tokio::test]
    async fn untrusted_connection_receives_no_projection() {
        let shared = make_runtime();
        let pm = make_pairing_manager();
        let (tx, _rx) = watch::channel(None);
        // Publish P1 before connection
        tx.send_replace(Some(Arc::from(
            "{\"type\":\"active_profile_state\",\"schema_version\":1,\"active_profile_id\":\"p1\"}",
        )));

        let requests = vec![identify("unknown-device")];
        let responses = run_test(
            shared,
            pm,
            tx,
            requests,
            std::time::Duration::from_millis(200),
        )
        .await;

        // Should receive untrusted — no active_profile_state
        assert!(!responses.is_empty(), "should receive untrusted");
        assert_eq!(responses[0]["type"], "untrusted");
        // Never received active_profile_state
        assert!(
            !responses
                .iter()
                .any(|r| r["type"] == "active_profile_state")
        );
    }

    #[tokio::test]
    async fn identified_but_untrusted_receives_no_projection() {
        let shared = make_runtime();
        let pm = make_pairing_manager();
        let (tx, _rx) = watch::channel(None);
        tx.send_replace(Some(Arc::from(
            "{\"type\":\"active_profile_state\",\"schema_version\":1,\"active_profile_id\":\"p1\"}",
        )));

        // identify then send ping (requires trust) — should fail, no projection
        let requests = vec![identify("unknown-device"), json!({"type": "ping"})];
        let responses = run_test(
            shared,
            pm,
            tx,
            requests,
            std::time::Duration::from_millis(200),
        )
        .await;

        assert!(responses.len() >= 1);
        assert_eq!(responses[0]["type"], "untrusted");
        assert!(
            !responses
                .iter()
                .any(|r| r["type"] == "active_profile_state")
        );
    }

    #[tokio::test]
    async fn trusted_ordering_trusted_before_active_profile_state() {
        let shared = make_runtime();
        add_trusted_device(&shared, "trusted-device");
        let pm = make_pairing_manager();
        let (tx, _rx) = watch::channel(None);
        tx.send_replace(Some(Arc::from(
            "{\"type\":\"active_profile_state\",\"schema_version\":1,\"active_profile_id\":\"p1\"}",
        )));

        let requests = vec![identify("trusted-device")];
        let responses = run_test(
            shared,
            pm,
            tx,
            requests,
            std::time::Duration::from_millis(200),
        )
        .await;

        assert!(
            responses.len() >= 2,
            "expected at least 2 responses, got {}",
            responses.len()
        );
        assert_eq!(responses[0]["type"], "trusted");
        assert_eq!(responses[1]["type"], "active_profile_state");
        assert_eq!(responses[1]["active_profile_id"], "p1");
    }

    #[tokio::test]
    async fn otp_pairing_ordered_pair_accepted_before_active_profile_state() {
        let shared = make_runtime();
        // Generate session on a fresh manager, then wrap it
        let mgr = PairingManager::new();
        let snap = mgr.generate_session("new-device", "test-host", 9742);
        let code = snap.otp.clone();
        let pm: SharedPairingManager = Arc::new(mgr);

        let (tx, _rx) = watch::channel(None);
        tx.send_replace(Some(Arc::from(
            "{\"type\":\"active_profile_state\",\"schema_version\":1,\"active_profile_id\":\"p1\"}",
        )));

        let requests = vec![identify("new-device"), pair_request_otp(&code)];
        let responses = run_test(
            shared,
            pm,
            tx,
            requests,
            std::time::Duration::from_millis(200),
        )
        .await;

        assert!(
            responses.len() >= 3,
            "expected at least 3 responses (untrusted + pair_accepted + active_profile_state), got {}: {:?}",
            responses.len(),
            responses
        );
        // After identify → untrusted (device not known)
        assert_eq!(responses[0]["type"], "untrusted");
        // After pair_request with valid OTP → pair_accepted
        assert_eq!(responses[1]["type"], "pair_accepted");
        // Then active_profile_state
        assert_eq!(responses[2]["type"], "active_profile_state");
        assert_eq!(responses[2]["active_profile_id"], "p1");
    }

    #[tokio::test]
    async fn watch_none_sends_no_snapshot() {
        let shared = make_runtime();
        add_trusted_device(&shared, "trusted-device");
        let (tx, _rx) = watch::channel(None);
        // No publish — value is None

        let requests = vec![identify("trusted-device")];
        let responses = run_test(
            shared,
            make_pairing_manager(),
            tx,
            requests,
            std::time::Duration::from_millis(200),
        )
        .await;

        assert!(responses.len() >= 1, "expected at least trusted response");
        assert_eq!(responses[0]["type"], "trusted");
        // No active_profile_state should follow from None
        assert!(
            !responses
                .iter()
                .any(|r| r["type"] == "active_profile_state")
        );
    }

    #[tokio::test]
    async fn null_profile_payload_delivered_unchanged() {
        let shared = make_runtime();
        add_trusted_device(&shared, "trusted-device");
        let pm = make_pairing_manager();
        let (tx, _rx) = watch::channel(None);
        // Publish a payload with active_profile_id: null
        let payload =
            "{\"type\":\"active_profile_state\",\"schema_version\":1,\"active_profile_id\":null}";
        tx.send_replace(Some(Arc::from(payload)));

        let requests = vec![identify("trusted-device")];
        let responses = run_test(
            shared,
            pm,
            tx,
            requests,
            std::time::Duration::from_millis(200),
        )
        .await;

        assert!(
            responses.len() >= 2,
            "expected trusted + active_profile_state"
        );
        assert_eq!(responses[0]["type"], "trusted");
        assert_eq!(responses[1]["type"], "active_profile_state");
        assert_eq!(responses[1]["active_profile_id"], serde_json::Value::Null);
    }

    #[tokio::test]
    async fn initial_p1_exactly_once() {
        let shared = make_runtime();
        add_trusted_device(&shared, "trusted-device");
        let pm = make_pairing_manager();
        let (tx, _rx) = watch::channel(None);
        tx.send_replace(Some(Arc::from(
            "{\"type\":\"active_profile_state\",\"schema_version\":1,\"active_profile_id\":\"p1\"}",
        )));

        let requests = vec![identify("trusted-device")];
        let responses = run_test(
            shared,
            pm,
            tx,
            requests,
            std::time::Duration::from_millis(200),
        )
        .await;

        let proj_count = responses
            .iter()
            .filter(|r| r["type"] == "active_profile_state")
            .count();
        assert_eq!(proj_count, 1, "P1 must be delivered exactly once");
    }

    #[tokio::test]
    async fn p2_live_push_after_snapshot() {
        let shared = make_runtime();
        add_trusted_device(&shared, "trusted-device");
        let pm = make_pairing_manager();
        let (tx, _rx) = watch::channel(None);
        tx.send_replace(Some(Arc::from(
            "{\"type\":\"active_profile_state\",\"schema_version\":1,\"active_profile_id\":\"p1\"}",
        )));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let url = format!("ws://{}/", addr);

        let pair_state: PairState = Arc::new(Mutex::new(None));
        let projection_rx = tx.subscribe();
        let (_css_tx, css_rx) = watch::channel(None);

        let server_handle = tokio::spawn(async move {
            let (tcp_stream, peer) = listener.accept().await.expect("accept");
            handle_connection(
                tcp_stream,
                peer,
                pair_state,
                make_pairing_manager(),
                shared,
                Arc::new(crate::repository::JsonRepository::new()),
                projection_rx,
                css_rx,
            )
            .await;
        });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let (mut ws_stream, _) = connect_async(&url).await.expect("connect");

        // Send identify as trusted device
        let identify_req = serde_json::to_string(&identify("trusted-device")).unwrap();
        ws_stream
            .send(Message::Text(identify_req.into()))
            .await
            .unwrap();

        // Read trusted + snapshot P1
        let mut responses = Vec::new();
        for _ in 0..2 {
            let msg = tokio::time::timeout(std::time::Duration::from_millis(500), ws_stream.next())
                .await
                .expect("timeout")
                .expect("some")
                .expect("ok");
            if let Message::Text(text) = msg {
                let val: serde_json::Value = serde_json::from_str(&text).unwrap();
                responses.push(val);
            }
        }

        assert_eq!(responses[0]["type"], "trusted");
        assert_eq!(responses[1]["type"], "active_profile_state");
        assert_eq!(responses[1]["active_profile_id"], "p1");

        // Now publish P2
        tx.send_replace(Some(Arc::from(
            "{\"type\":\"active_profile_state\",\"schema_version\":1,\"active_profile_id\":\"p2\"}",
        )));

        // Read P2 push
        let msg = tokio::time::timeout(std::time::Duration::from_millis(500), ws_stream.next())
            .await
            .expect("p2 timeout")
            .expect("some")
            .expect("ok");
        let mut push_responses = Vec::new();
        if let Message::Text(text) = msg {
            let val: serde_json::Value = serde_json::from_str(&text).unwrap();
            push_responses.push(val);
        }

        assert_eq!(push_responses[0]["type"], "active_profile_state");
        assert_eq!(push_responses[0]["active_profile_id"], "p2");

        drop(ws_stream);
        let _ = tokio::time::timeout(std::time::Duration::from_secs(1), server_handle).await;
    }

    #[tokio::test]
    async fn reconnecting_trusted_device_gets_latest_snapshot() {
        let shared = make_runtime();
        add_trusted_device(&shared, "trusted-device");
        let pm = make_pairing_manager();
        let (tx, _rx) = watch::channel(None);
        tx.send_replace(Some(Arc::from(
            "{\"type\":\"active_profile_state\",\"schema_version\":1,\"active_profile_id\":\"p1\"}",
        )));

        // First connection
        let requests = vec![identify("trusted-device")];
        let responses = run_test(
            shared.clone(),
            pm.clone(),
            tx.clone(),
            requests,
            std::time::Duration::from_millis(200),
        )
        .await;
        assert_eq!(responses[0]["type"], "trusted");
        assert_eq!(responses[1]["active_profile_id"], "p1");

        // Publish P2
        tx.send_replace(Some(Arc::from(
            "{\"type\":\"active_profile_state\",\"schema_version\":1,\"active_profile_id\":\"p2\"}",
        )));

        // Reconnect — should see P2, not P1
        let requests = vec![identify("trusted-device")];
        let responses = run_test(
            shared,
            pm,
            tx,
            requests,
            std::time::Duration::from_millis(200),
        )
        .await;
        assert_eq!(responses[0]["type"], "trusted");
        assert_eq!(responses[1]["active_profile_id"], "p2");
    }

    #[tokio::test]
    async fn rapid_publication_coalesces_to_latest() {
        let shared = make_runtime();
        add_trusted_device(&shared, "trusted-device");
        let pm = make_pairing_manager();
        let (tx, _rx) = watch::channel(None);
        tx.send_replace(Some(Arc::from(
            "{\"type\":\"active_profile_state\",\"schema_version\":1,\"active_profile_id\":\"p1\"}",
        )));

        // Connect, get snapshot P1
        let requests = vec![identify("trusted-device")];
        let responses = run_test(
            shared.clone(),
            pm.clone(),
            tx.clone(),
            requests,
            std::time::Duration::from_millis(200),
        )
        .await;
        assert_eq!(responses[1]["active_profile_id"], "p1");

        // Publish P2, P3, P4 very fast
        tx.send_replace(Some(Arc::from(
            "{\"type\":\"active_profile_state\",\"schema_version\":1,\"active_profile_id\":\"p2\"}",
        )));
        tx.send_replace(Some(Arc::from(
            "{\"type\":\"active_profile_state\",\"schema_version\":1,\"active_profile_id\":\"p3\"}",
        )));
        tx.send_replace(Some(Arc::from(
            "{\"type\":\"active_profile_state\",\"schema_version\":1,\"active_profile_id\":\"p4\"}",
        )));

        // Use the full harness for the live push variant — but this tests
        // via the live push path that already exists from p1 connection.
        // For coalescing, we need a second connection after P4 only:
        let requests = vec![identify("trusted-device")];
        let responses = run_test(
            shared,
            pm,
            tx,
            requests,
            std::time::Duration::from_millis(200),
        )
        .await;
        assert_eq!(responses[1]["active_profile_id"], "p4");
    }

    #[tokio::test]
    async fn one_connection_write_failure_does_not_affect_others() {
        let shared = make_runtime();
        add_trusted_device(&shared, "trusted-device");
        let pm = make_pairing_manager();
        let (tx, _rx) = watch::channel(None);
        let payload =
            "{\"type\":\"active_profile_state\",\"schema_version\":1,\"active_profile_id\":\"p1\"}";
        tx.send_replace(Some(Arc::from(payload)));

        // Two separate listeners for two connections
        let l1 = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr1 = l1.local_addr().unwrap();
        let url1 = format!("ws://{}/", addr1);

        let l2 = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr2 = l2.local_addr().unwrap();
        let url2 = format!("ws://{}/", addr2);

        let shared2 = shared.clone();
        let pm2 = pm.clone();

        let pair_state1: PairState = Arc::new(Mutex::new(None));
        let pair_state2: PairState = Arc::new(Mutex::new(None));
        let rx1 = tx.subscribe();
        let rx2 = tx.subscribe();
        let (_css_tx, css_rx1) = watch::channel(None);
        let css_rx2 = css_rx1.clone();
        let repo = Arc::new(crate::repository::JsonRepository::new());
        let repo2 = repo.clone();

        let h1 = tokio::spawn(async move {
            let (tcp, peer) = l1.accept().await.unwrap();
            handle_connection(tcp, peer, pair_state1, pm, shared, repo, rx1, css_rx1).await;
        });
        let h2 = tokio::spawn(async move {
            let (tcp, peer) = l2.accept().await.unwrap();
            handle_connection(tcp, peer, pair_state2, pm2, shared2, repo2, rx2, css_rx2).await;
        });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let (mut ws1, _) = connect_async(&url1).await.unwrap();
        let (mut ws2, _) = connect_async(&url2).await.unwrap();

        // Both identify
        let id_req = serde_json::to_string(&identify("trusted-device")).unwrap();
        ws1.send(Message::Text(id_req.clone().into()))
            .await
            .unwrap();
        ws2.send(Message::Text(id_req.into())).await.unwrap();

        // Both read trusted + snapshot
        for _ in 0..2 {
            let _ = tokio::time::timeout(std::time::Duration::from_millis(500), ws1.next()).await;
            let _ = tokio::time::timeout(std::time::Duration::from_millis(500), ws2.next()).await;
        }

        // Close ws1 — its connection handler will see read error and exit
        drop(ws1);

        // Give time for the handler to clean up
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // Publish P2
        tx.send_replace(Some(Arc::from(
            "{\"type\":\"active_profile_state\",\"schema_version\":1,\"active_profile_id\":\"p2\"}",
        )));

        // ws2 should still receive P2
        let msg = tokio::time::timeout(std::time::Duration::from_millis(500), ws2.next())
            .await
            .expect("ws2 should still receive")
            .expect("some")
            .expect("ok");
        if let Message::Text(text) = msg {
            let val: serde_json::Value = serde_json::from_str(&text).unwrap();
            assert_eq!(val["type"], "active_profile_state");
            assert_eq!(val["active_profile_id"], "p2");
        }

        drop(ws2);
        drop(tx);
        let _ = tokio::time::timeout(std::time::Duration::from_secs(1), h1).await;
        let _ = tokio::time::timeout(std::time::Duration::from_secs(1), h2).await;
    }

    #[tokio::test]
    async fn rapid_publication_live_push_coalesces() {
        let shared = make_runtime();
        add_trusted_device(&shared, "trusted-device");
        let pm = make_pairing_manager();
        let (tx, _rx) = watch::channel(None);
        tx.send_replace(Some(Arc::from(
            "{\"type\":\"active_profile_state\",\"schema_version\":1,\"active_profile_id\":\"p1\"}",
        )));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let url = format!("ws://{}/", addr);

        let pair_state: PairState = Arc::new(Mutex::new(None));
        let projection_rx = tx.subscribe();
        let (_css_tx, css_rx) = watch::channel(None);

        let server_handle = tokio::spawn(async move {
            let (tcp_stream, peer) = listener.accept().await.expect("accept");
            handle_connection(
                tcp_stream,
                peer,
                pair_state,
                pm,
                shared,
                Arc::new(crate::repository::JsonRepository::new()),
                projection_rx,
                css_rx,
            )
            .await;
        });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let (mut ws_stream, _) = connect_async(&url).await.expect("connect");

        // Identify as trusted
        let id_req = serde_json::to_string(&identify("trusted-device")).unwrap();
        ws_stream.send(Message::Text(id_req.into())).await.unwrap();

        // Read trusted + P1 snapshot
        let _ = tokio::time::timeout(std::time::Duration::from_millis(500), ws_stream.next()).await;
        let _ = tokio::time::timeout(std::time::Duration::from_millis(500), ws_stream.next()).await;

        // Rapid-fire publishes
        tx.send_replace(Some(Arc::from(
            "{\"type\":\"active_profile_state\",\"schema_version\":1,\"active_profile_id\":\"p2\"}",
        )));
        tx.send_replace(Some(Arc::from(
            "{\"type\":\"active_profile_state\",\"schema_version\":1,\"active_profile_id\":\"p3\"}",
        )));
        tx.send_replace(Some(Arc::from(
            "{\"type\":\"active_profile_state\",\"schema_version\":1,\"active_profile_id\":\"p4\"}",
        )));

        // Read — should converge on P4 (may also receive P2/P3 depending on timing)
        let msg = tokio::time::timeout(std::time::Duration::from_millis(500), ws_stream.next())
            .await
            .expect("timeout")
            .expect("some")
            .expect("ok");
        if let Message::Text(text) = msg {
            let val: serde_json::Value = serde_json::from_str(&text).unwrap();
            assert_eq!(val["type"], "active_profile_state");
            // Must converge to P4 (latest)
            assert_eq!(val["active_profile_id"], "p4");
        }

        drop(ws_stream);
        let _ = tokio::time::timeout(std::time::Duration::from_secs(1), server_handle).await;
    }

    // ── CSS WebSocket transport tests ──

    #[tokio::test]
    async fn css_live_push_after_trust() {
        let shared = make_runtime();
        add_trusted_device(&shared, "trusted-device");
        let pm = make_pairing_manager();
        let (aps_tx, _rx) = watch::channel(None);
        let (css_tx, css_rx) = watch::channel::<Option<Arc<str>>>(None);
        aps_tx.send_replace(Some(Arc::from(
            "{\"type\":\"active_profile_state\",\"schema_version\":1,\"active_profile_id\":\"p1\"}",
        )));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let url = format!("ws://{}/", addr);

        let pair_state: PairState = Arc::new(Mutex::new(None));
        let projection_rx = aps_tx.subscribe();
        let css_rx = css_rx;

        let server_handle = tokio::spawn(async move {
            let (tcp_stream, peer) = listener.accept().await.expect("accept");
            handle_connection(
                tcp_stream,
                peer,
                pair_state,
                pm,
                shared,
                Arc::new(crate::repository::JsonRepository::new()),
                projection_rx,
                css_rx,
            )
            .await;
        });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let (mut ws_stream, _) = connect_async(&url).await.expect("connect");

        // Identify as trusted — get trusted + APS snapshot
        let id_req = serde_json::to_string(&identify("trusted-device")).unwrap();
        ws_stream.send(Message::Text(id_req.into())).await.unwrap();

        // Consume trusted + APS
        let _trusted = recv_json(&mut ws_stream, std::time::Duration::from_millis(500)).await;
        let _aps = recv_json(&mut ws_stream, std::time::Duration::from_millis(500)).await;

        // NOW publish a CSS payload after trust is established
        let css_payload =
            make_css_payload("prof-1", "Gaming", "pg-1", "Desktop", "btn-a", "Launch");
        css_tx.send_replace(Some(Arc::from(css_payload)));

        // Receive CSS via live forwarding (the changed() branch)
        let css = recv_message_type(
            &mut ws_stream,
            "control_surface_state",
            std::time::Duration::from_millis(500),
            10,
        )
        .await;

        assert_eq!(css["schema_version"], 1);
        assert_eq!(css["profile_id"], "prof-1");
        assert_eq!(css["profile_name"], "Gaming");
        assert_eq!(css["pages"][0]["page_id"], "pg-1");
        assert_eq!(css["pages"][0]["name"], "Desktop");
        assert_eq!(css["pages"][0]["buttons"][0]["button_id"], "btn-a");
        assert_eq!(css["pages"][0]["buttons"][0]["label"], "Launch");

        drop(ws_stream);
        let _ = tokio::time::timeout(std::time::Duration::from_secs(1), server_handle).await;
    }

    #[tokio::test]
    async fn css_retained_snapshot_delivered_after_trust() {
        let shared = make_runtime();
        add_trusted_device(&shared, "trusted-device");
        let pm = make_pairing_manager();
        let (aps_tx, _rx) = watch::channel(None);
        aps_tx.send_replace(Some(Arc::from(
            "{\"type\":\"active_profile_state\",\"schema_version\":1,\"active_profile_id\":\"p1\"}",
        )));

        // CSS payload retained BEFORE connection
        let css_payload = make_css_payload("prof-2", "Coding", "pg-2", "Dev", "btn-b", "Build");
        let (css_tx, css_rx) = watch::channel(Some(Arc::from(css_payload)));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let url = format!("ws://{}/", addr);

        let pair_state: PairState = Arc::new(Mutex::new(None));
        let projection_rx = aps_tx.subscribe();

        let server_handle = tokio::spawn(async move {
            let (tcp_stream, peer) = listener.accept().await.expect("accept");
            handle_connection(
                tcp_stream,
                peer,
                pair_state,
                pm,
                shared,
                Arc::new(crate::repository::JsonRepository::new()),
                projection_rx,
                css_rx,
            )
            .await;
        });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let (mut ws_stream, _) = connect_async(&url).await.expect("connect");

        // Identify — no css_tx.send() after this point
        let id_req = serde_json::to_string(&identify("trusted-device")).unwrap();
        ws_stream.send(Message::Text(id_req.into())).await.unwrap();

        // Find the retained CSS message (must arrive without post-trust send)
        let css = recv_message_type(
            &mut ws_stream,
            "control_surface_state",
            std::time::Duration::from_millis(500),
            10,
        )
        .await;

        assert_eq!(css["schema_version"], 1);
        assert_eq!(css["profile_id"], "prof-2");
        assert_eq!(css["profile_name"], "Coding");
        assert_eq!(css["pages"][0]["page_id"], "pg-2");

        // Verify no css_tx.send() happened — this would be caught by the
        // fact that we never call send_replace on css_tx after this point
        drop(css_tx);

        drop(ws_stream);
        let _ = tokio::time::timeout(std::time::Duration::from_secs(1), server_handle).await;
    }

    #[tokio::test]
    async fn css_channel_closure_does_not_stop_aps_forwarding() {
        let shared = make_runtime();
        add_trusted_device(&shared, "trusted-device");
        let pm = make_pairing_manager();
        let (aps_tx, _rx) = watch::channel(None);
        let (css_tx, css_rx) = watch::channel::<Option<Arc<str>>>(None);
        aps_tx.send_replace(Some(Arc::from(
            "{\"type\":\"active_profile_state\",\"schema_version\":1,\"active_profile_id\":\"p1\"}",
        )));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let url = format!("ws://{}/", addr);

        let pair_state: PairState = Arc::new(Mutex::new(None));
        let projection_rx = aps_tx.subscribe();

        let server_handle = tokio::spawn(async move {
            let (tcp_stream, peer) = listener.accept().await.expect("accept");
            handle_connection(
                tcp_stream,
                peer,
                pair_state,
                pm,
                shared,
                Arc::new(crate::repository::JsonRepository::new()),
                projection_rx,
                css_rx,
            )
            .await;
        });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let (mut ws_stream, _) = connect_async(&url).await.expect("connect");

        // Identify as trusted
        let id_req = serde_json::to_string(&identify("trusted-device")).unwrap();
        ws_stream.send(Message::Text(id_req.into())).await.unwrap();

        // Consume trusted + APS
        let _trusted = recv_json(&mut ws_stream, std::time::Duration::from_millis(500)).await;
        let _aps = recv_json(&mut ws_stream, std::time::Duration::from_millis(500)).await;

        // Drop the CSS sender — disables only CSS branch
        drop(css_tx);

        // Publish a new APS payload
        aps_tx.send_replace(Some(Arc::from(
            "{\"type\":\"active_profile_state\",\"schema_version\":1,\"active_profile_id\":\"p2\"}",
        )));

        // Client must still receive APS even though CSS channel is closed
        let aps = recv_message_type(
            &mut ws_stream,
            "active_profile_state",
            std::time::Duration::from_millis(500),
            10,
        )
        .await;

        assert_eq!(aps["active_profile_id"], "p2");

        drop(ws_stream);
        let _ = tokio::time::timeout(std::time::Duration::from_secs(1), server_handle).await;
    }

    #[tokio::test]
    async fn aps_channel_closure_does_not_stop_css_forwarding() {
        let shared = make_runtime();
        add_trusted_device(&shared, "trusted-device");
        let pm = make_pairing_manager();
        let (aps_tx, _rx) = watch::channel(None);
        let (css_tx, css_rx) = watch::channel::<Option<Arc<str>>>(None);
        aps_tx.send_replace(Some(Arc::from(
            "{\"type\":\"active_profile_state\",\"schema_version\":1,\"active_profile_id\":\"p1\"}",
        )));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let url = format!("ws://{}/", addr);

        let pair_state: PairState = Arc::new(Mutex::new(None));
        let projection_rx = aps_tx.subscribe();

        let server_handle = tokio::spawn(async move {
            let (tcp_stream, peer) = listener.accept().await.expect("accept");
            handle_connection(
                tcp_stream,
                peer,
                pair_state,
                pm,
                shared,
                Arc::new(crate::repository::JsonRepository::new()),
                projection_rx,
                css_rx,
            )
            .await;
        });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let (mut ws_stream, _) = connect_async(&url).await.expect("connect");

        // Identify as trusted
        let id_req = serde_json::to_string(&identify("trusted-device")).unwrap();
        ws_stream.send(Message::Text(id_req.into())).await.unwrap();

        // Consume trusted + APS
        let _trusted = recv_json(&mut ws_stream, std::time::Duration::from_millis(500)).await;
        let _aps = recv_json(&mut ws_stream, std::time::Duration::from_millis(500)).await;

        // Drop the APS sender — disables only APS branch
        drop(aps_tx);

        // Publish a new CSS payload
        let css_payload = make_css_payload("prof-3", "Design", "pg-3", "UI", "btn-c", "Render");
        css_tx.send_replace(Some(Arc::from(css_payload)));

        // Client must still receive CSS even though APS channel is closed
        let css = recv_message_type(
            &mut ws_stream,
            "control_surface_state",
            std::time::Duration::from_millis(500),
            10,
        )
        .await;

        assert_eq!(css["schema_version"], 1);
        assert_eq!(css["profile_id"], "prof-3");
        assert_eq!(css["profile_name"], "Design");
        assert_eq!(css["pages"][0]["buttons"][0]["label"], "Render");

        drop(ws_stream);
        let _ = tokio::time::timeout(std::time::Duration::from_secs(1), server_handle).await;
    }
}
