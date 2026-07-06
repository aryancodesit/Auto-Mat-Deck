mod discovery;

use std::net::SocketAddr;

use discovery::AdvertisementProvider;
use futures_util::{SinkExt, StreamExt};
use log::{info, warn};
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;

const PORT: u16 = 9742;

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

async fn handle_connection(
    stream: tokio::net::TcpStream,
    peer: SocketAddr,
) {
    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            warn!("WebSocket handshake failed from {}: {}", peer, e);
            return;
        }
    };

    let (mut write, mut read) = ws_stream.split();

    while let Some(msg) = read.next().await {
        match msg {
            Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
                match serde_json::from_str::<Value>(&text) {
                    Ok(req) => {
                        let msg_type = req
                            .get("type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown");

                        match msg_type {
                            "ping" => {
                                let resp = json!({
                                    "type": "pong",
                                    "echo": req,
                                    "deviceId": peer.to_string()
                                });
                                let resp_text = serde_json::to_string(&resp).unwrap();
                                if let Err(e) =
                                    write.send(tokio_tungstenite::tungstenite::Message::Text(
                                        resp_text.into(),
                                    )).await
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
                                let resp_text = serde_json::to_string(&resp).unwrap();
                                let _ = write
                                    .send(tokio_tungstenite::tungstenite::Message::Text(
                                        resp_text.into(),
                                    )).await;
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Invalid JSON from {}: {}", peer, e);
                        let resp = json!({
                            "type": "error",
                            "message": "Invalid JSON"
                        });
                        let resp_text = serde_json::to_string(&resp).unwrap();
                        let _ = write
                            .send(tokio_tungstenite::tungstenite::Message::Text(
                                resp_text.into(),
                            )).await;
                    }
                }
            }
            Ok(tokio_tungstenite::tungstenite::Message::Close(_)) => {
                info!("Connection closed by {}", peer);
                break;
            }
            Ok(tokio_tungstenite::tungstenite::Message::Ping(data)) => {
                let _ = write
                    .send(tokio_tungstenite::tungstenite::Message::Pong(data))
                    .await;
            }
            Err(e) => {
                warn!("WebSocket error from {}: {}", peer, e);
                break;
            }
            _ => {}
        }
    }

    info!("Connection closed: {}", peer);
}
