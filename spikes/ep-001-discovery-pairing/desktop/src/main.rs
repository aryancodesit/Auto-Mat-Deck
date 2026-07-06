mod discovery;

use std::io::Write;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use discovery::AdvertisementProvider;
use futures_util::{SinkExt, StreamExt};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

const PORT: u16 = 9742;

// --- Trusted device store ---

#[derive(Serialize, Deserialize, Clone)]
struct TrustedDevice {
    device_id: String,
    device_name: String,
    last_seen: u64,
    paired_at: u64,
}

fn get_data_dir() -> PathBuf {
    std::env::var("APPDATA")
        .map(|p| PathBuf::from(p).join("AutoMatDeck"))
        .unwrap_or_else(|_| PathBuf::from("data"))
}

fn store_path() -> PathBuf {
    get_data_dir().join("trusted_devices.json")
}

fn load_devices() -> Vec<TrustedDevice> {
    let path = store_path();
    if path.exists() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        Vec::new()
    }
}

fn save_devices(devices: &[TrustedDevice]) {
    let path = store_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    if let Ok(content) = serde_json::to_string_pretty(devices) {
        std::fs::write(&path, &content).ok();
    }
}

fn is_trusted(device_id: &str) -> bool {
    load_devices().iter().any(|d| d.device_id == device_id)
}

fn add_device(device_id: &str, device_name: &str) {
    let mut devices = load_devices();
    devices.retain(|d| d.device_id != device_id);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    devices.push(TrustedDevice {
        device_id: device_id.to_string(),
        device_name: device_name.to_string(),
        last_seen: now,
        paired_at: now,
    });
    save_devices(&devices);
}

fn touch_device(device_id: &str) {
    let mut devices = load_devices();
    if let Some(d) = devices.iter_mut().find(|d| d.device_id == device_id) {
        d.last_seen = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        save_devices(&devices);
    }
}

// --- Console approval (async, non-blocking) ---

async fn prompt_approval(device_name: &str, device_id: &str) -> bool {
    println!("\n=== Pairing Request ===");
    println!("Device: {} ({})", device_name, device_id);
    print!("Accept? [y/N]: ");
    std::io::stdout().flush().ok();

    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    if let Ok(Some(line)) = lines.next_line().await {
        line.trim().eq_ignore_ascii_case("y")
    } else {
        false
    }
}

// --- WebSocket handler ---

async fn handle_connection(stream: tokio::net::TcpStream, peer: SocketAddr) {
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

                            if is_trusted(&id) {
                                trusted = true;
                                touch_device(&id);
                                info!("Trusted device connected: {} ({})", name, id);
                                let resp = json!({"type": "trusted", "device_id": id});
                                let _ = write
                                    .send(Message::Text(resp.to_string().into()))
                                    .await;
                            } else {
                                info!("Unknown device connected: {} ({})", name, id);
                                let resp = json!({
                                    "type": "untrusted",
                                    "message": "Device not paired. Send pair_request to initiate pairing."
                                });
                                let _ = write
                                    .send(Message::Text(resp.to_string().into()))
                                    .await;
                            }
                        }

                        "pair_request" => {
                            if trusted {
                                let resp = json!({"type": "error", "message": "Already paired"});
                                let _ = write
                                    .send(Message::Text(resp.to_string().into()))
                                    .await;
                                continue;
                            }

                            let id = match device_id {
                                Some(ref id) => id.clone(),
                                None => {
                                    let resp = json!({
                                        "type": "error",
                                        "message": "Must identify first"
                                    });
                                    let _ = write
                                        .send(Message::Text(resp.to_string().into()))
                                        .await;
                                    continue;
                                }
                            };

                            let device_name = req
                                .get("device_name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown");

                            let approved = prompt_approval(device_name, &id).await;

                            if approved {
                                add_device(&id, device_name);
                                trusted = true;
                                info!("Paired with device: {} ({})", device_name, id);
                                let resp = json!({"type": "pair_accepted", "device_id": id});
                                let _ = write
                                    .send(Message::Text(resp.to_string().into()))
                                    .await;
                            } else {
                                info!("Pairing rejected for: {} ({})", device_name, id);
                                let resp = json!({
                                    "type": "pair_rejected",
                                    "device_id": id,
                                    "reason": "User declined"
                                });
                                let _ = write
                                    .send(Message::Text(resp.to_string().into()))
                                    .await;
                            }
                        }

                        "ping" => {
                            if !trusted && device_id.is_some() {
                                let resp = json!({
                                    "type": "error",
                                    "message": "Device not trusted. Complete pairing first."
                                });
                                let _ = write
                                    .send(Message::Text(resp.to_string().into()))
                                    .await;
                                continue;
                            }
                            let resp = json!({
                                "type": "pong",
                                "echo": req,
                                "deviceId": peer.to_string()
                            });
                            let resp_text = serde_json::to_string(&resp).unwrap();
                            if let Err(e) =
                                write.send(Message::Text(resp_text.into())).await
                            {
                                warn!("Failed to send pong to {}: {}", peer, e);
                                break;
                            }
                        }

                        _ => {
                            let resp = json!({
                                "type": "error",
                                "message": format!("Unknown message type: {}", msg_type)
                            });
                            let _ = write
                                .send(Message::Text(resp.to_string().into()))
                                .await;
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

// --- Main ---

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    let device_id = format!("amd-{}", &hostname);

    let mut advertisers: Vec<Box<dyn AdvertisementProvider>> = Vec::new();
    advertisers.push(Box::new(discovery::MdnsAnnouncer::new(
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
    info!("Trusted devices stored at: {}", store_path().display());

    let addr: SocketAddr = ([0, 0, 0, 0], PORT).into();
    let listener = TcpListener::bind(&addr)
        .await
        .expect("Failed to bind WebSocket server");

    info!("WebSocket server listening on ws://{}", addr);

    while let Ok((stream, peer)) = listener.accept().await {
        info!("New connection from {}", peer);
        tokio::spawn(handle_connection(stream, peer));
    }
}
