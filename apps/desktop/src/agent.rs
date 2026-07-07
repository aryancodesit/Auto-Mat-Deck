use std::net::SocketAddr;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;

use crate::actions::ActionRegistry;
use crate::device_store;

use futures_util::{SinkExt, StreamExt};
use log::{info, warn};
use serde_json::{json, Value};
use tokio::net::TcpListener;
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
    mut shutdown_rx: tokio::sync::watch::Receiver<bool>,
    pair_state: PairState,
) {
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    let device_id = format!("amd-{}", &hostname);

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
            Err(e) => warn!(
                "[{}] Failed to start: {}",
                advertiser.provider_name(),
                e
            ),
        }
    }

    info!(
        "Desktop agent started. Hostname: {}, Device ID: {}, Listening on port {}",
        hostname, device_id, PORT
    );

    let addr: SocketAddr = ([0, 0, 0, 0], PORT).into();
    let listener = TcpListener::bind(&addr)
        .await
        .expect("Failed to bind WebSocket server");

    info!("WebSocket server listening on ws://{}", addr);

    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, peer)) => {
                        info!("New connection from {}", peer);
                        tokio::spawn(handle_connection(stream, peer, pair_state.clone()));
                    }
                    Err(e) => {
                        warn!("Accept error: {}", e);
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

async fn handle_connection(
    stream: tokio::net::TcpStream,
    peer: SocketAddr,
    pair_state: PairState,
) {
    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            warn!("WebSocket handshake failed from {}: {}", peer, e);
            return;
        }
    };

    let (mut write, mut read) = ws_stream.split();
    let mut trusted = false;
    let mut device_id: Option<String> = None;

    while let Some(msg) = read.next().await {
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

                            if device_store::is_trusted(&id) {
                                trusted = true;
                                device_store::touch_device(&id);
                                info!("Trusted device connected: {} ({})", name, id);
                                let resp = json!({"type": "trusted", "device_id": id});
                                let _ = write.send(Message::Text(resp.to_string().into())).await;
                            } else {
                                info!("Unknown device connected: {} ({})", name, id);
                                let resp = json!({
                                    "type": "untrusted",
                                    "message": "Device not paired. Send pair_request to initiate pairing."
                                });
                                let _ = write.send(Message::Text(resp.to_string().into())).await;
                            }
                        }

                        "pair_request" => {
                            if trusted {
                                let resp = json!({"type": "error", "message": "Already paired"});
                                let _ = write.send(Message::Text(resp.to_string().into())).await;
                                continue;
                            }

                            let id = match device_id {
                                Some(ref id) => id.clone(),
                                None => {
                                    let resp = json!({"type": "error", "message": "Must identify first"});
                                    let _ = write.send(Message::Text(resp.to_string().into())).await;
                                    continue;
                                }
                            };

                            let device_name = req
                                .get("device_name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                                .to_string();

                            let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();

                            {
                                let mut state = pair_state.lock().unwrap();
                                *state = Some(PendingPair {
                                    device_id: id.clone(),
                                    device_name: device_name.clone(),
                                    responder: resp_tx,
                                });
                            }
                            info!("Pair request from {} ({}), awaiting approval", device_name, id);

                            let approved = match tokio::time::timeout(
                                Duration::from_secs(PAIR_TIMEOUT_SECS),
                                resp_rx,
                            )
                            .await
                            {
                                Ok(Ok(true)) => true,
                                _ => false,
                            };

                            {
                                let mut state = pair_state.lock().unwrap();
                                *state = None;
                            }

                            if approved {
                                device_store::add_device(&id, &device_name);
                                trusted = true;
                                info!("Paired with device: {} ({})", device_name, id);
                                let resp = json!({"type": "pair_accepted", "device_id": id});
                                let _ = write.send(Message::Text(resp.to_string().into())).await;
                            } else {
                                info!("Pairing rejected for: {} ({})", device_name, id);
                                let resp = json!({
                                    "type": "pair_rejected",
                                    "device_id": id,
                                    "reason": "User declined or timeout"
                                });
                                let _ = write.send(Message::Text(resp.to_string().into())).await;
                            }
                        }

                        "ping" => {
                            if !trusted && device_id.is_some() {
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
                            if !trusted {
                                let rid = req.get("request_id").and_then(|v| v.as_str()).unwrap_or("unknown");
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

                            let action_name = req
                                .get("action")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");

                            let empty = json!({});
                            let payload = req.get("payload").unwrap_or(&empty);

                            info!("Action from {}: action={}, request_id={}", peer, action_name, request_id);

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

                            if let Err(e) = write.send(Message::Text(resp.to_string().into())).await {
                                warn!("Failed to send action result to {}: {}", peer, e);
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
