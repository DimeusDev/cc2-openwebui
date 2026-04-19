use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::State;
use axum::response::IntoResponse;
use futures::{SinkExt, StreamExt};
use serde_json::json;
use tokio::sync::{broadcast, Mutex, RwLock};
use tracing::{debug, info, warn};

use super::router::AppState;
use crate::printer::state::{PrinterEvent, PrinterState};

pub async fn ws_handler(
    State(state): State<AppState>,
    ws: axum::extract::WebSocketUpgrade,
) -> impl IntoResponse {
    let (event_rx, state_changed_rx) = {
        let manager = state.manager.lock().await;
        (manager.event_rx(), manager.state_changed_rx())
    };
    ws.on_upgrade(move |socket| {
        handle_socket(socket, state.printer_state.clone(), event_rx, state_changed_rx)
    })
}

async fn handle_socket(
    socket: WebSocket,
    printer_state: Arc<RwLock<PrinterState>>,
    event_rx: broadcast::Receiver<PrinterEvent>,
    state_changed_rx: broadcast::Receiver<()>,
) {
    let (ws_sender, mut ws_receiver) = socket.split();
    let sender = Arc::new(Mutex::new(ws_sender));

    info!("websocket client connected");

    {
        let s = printer_state.read().await;
        let msg = build_state_msg(&s);
        if let Ok(text) = serde_json::to_string(&msg) {
            let mut sender = sender.lock().await;
            if let Err(e) = sender.send(Message::Text(text)).await {
                info!("[ws] failed to send initial state: {e}");
                return;
            }
        }
    }

    let sender_clone = sender.clone();
    let printer_state_clone = printer_state.clone();

    let send_task = tokio::spawn(async move {
        let mut event_stream = event_rx;
        let mut state_changed = state_changed_rx;
        let mut fallback = tokio::time::interval(std::time::Duration::from_secs(30));
        fallback.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                result = state_changed.recv() => {
                    match result {
                        Ok(()) | Err(broadcast::error::RecvError::Lagged(_)) => {
                            let s = printer_state_clone.read().await;
                            let msg = build_state_msg(&s);
                            if let Ok(text) = serde_json::to_string(&msg) {
                                let mut sender = sender_clone.lock().await;
                                if sender.send(Message::Text(text)).await.is_err() {
                                    break;
                                }
                            }
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            warn!("[ws] state_changed channel closed");
                            break;
                        }
                    }
                }

                result = event_stream.recv() => {
                    match result {
                        Ok(event) => {
                            let msg = json!({
                                "type": "event",
                                "data": {
                                    "kind": format!("{:?}", event.kind),
                                    "description": event.description,
                                }
                            });
                            if let Ok(text) = serde_json::to_string(&msg) {
                                let mut sender = sender_clone.lock().await;
                                if sender.send(Message::Text(text)).await.is_err() {
                                    break;
                                }
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => {
                            debug!("[ws] event receiver lagged");
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            warn!("[ws] event channel closed");
                            break;
                        }
                    }
                }

                _ = fallback.tick() => {
                    let s = printer_state_clone.read().await;
                    let msg = build_state_msg(&s);
                    if let Ok(text) = serde_json::to_string(&msg) {
                        let mut sender = sender_clone.lock().await;
                        if sender.send(Message::Text(text)).await.is_err() {
                            break;
                        }
                    }
                }
            }
        }
    });

    while let Some(Ok(msg)) = ws_receiver.next().await {
        match msg {
            Message::Text(text) => {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                    if value.get("type").and_then(|t| t.as_str()) == Some("ping") {
                        let pong = json!({"type": "pong"});
                        if let Ok(text) = serde_json::to_string(&pong) {
                            let mut s = sender.lock().await;
                            if s.send(Message::Text(text)).await.is_err() {
                                break;
                            }
                        }
                    }
                }
            }
            Message::Close(_) => {
                debug!("[ws] client sent close frame");
                break;
            }
            _ => {}
        }
    }

    send_task.abort();
    info!("websocket client disconnected");
}

fn build_state_msg(s: &PrinterState) -> serde_json::Value {
    let data = serde_json::to_value(&s.full).unwrap_or(json!({}));
    let det_history: Vec<_> = s.detection_history.iter().collect();
    json!({
        "type": "state",
        "connected": s.connected,
        "printer_ip": s.printer_ip,
        "data": data,
        "detection_score": s.detection_score,
        "detection_history": det_history,
        "files": s.files,
    })
}
