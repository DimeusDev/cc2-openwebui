use std::sync::Arc;
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::net::TcpStream;
use tokio::sync::{broadcast, watch, RwLock};
use tokio::time::{interval, timeout};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tracing::{debug, error, info, trace, warn};

use super::commands::{Command, PendingRpcs};
use super::models::{FullStatus, RpcResponse, METHOD_GET_AMS_INFO, METHOD_GET_FILE_LIST, METHOD_GET_FULL_STATUS, METHOD_STATUS_PUSH};
use super::state::PrinterState;
use crate::error::PrinterError;

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub struct MqttWsClient;

impl MqttWsClient {
    pub async fn connect_and_run(
        ip: &str,
        printer_id: &str,
        username: &str,
        password: &str,
        client_id: &str,
        state: Arc<RwLock<PrinterState>>,
        connected_tx: watch::Sender<bool>,
        state_changed_tx: broadcast::Sender<()>,
        cmd_rx: broadcast::Receiver<Command>,
        mut shutdown: watch::Receiver<bool>,
        pending_rpcs: PendingRpcs,
    ) -> Result<(), PrinterError> {
        info!("ws client connecting to {ip}:9001");

        let url = format!("ws://{ip}:9001/");
        let mut request = url
            .into_client_request()
            .map_err(|e| PrinterError::WebSocket(format!("invalid ws url: {e}")))?;

        let proto_value = "mqttv3.1".parse().map_err(|e| {
            PrinterError::WebSocket(format!("invalid ws subprotocol header: {e}"))
        })?;
        request.headers_mut().insert("Sec-WebSocket-Protocol", proto_value);

        let (ws_stream, _) = tokio_tungstenite::connect_async_with_config(request, None, true)
            .await
            .map_err(|e| PrinterError::WebSocket(format!("ws connection failed: {e}")))?;

        info!("ws client TCP connected, starting MQTT session");

        Self::run_session(
            ws_stream,
            printer_id,
            client_id,
            username,
            password,
            state,
            connected_tx,
            state_changed_tx,
            cmd_rx,
            &mut shutdown,
            pending_rpcs,
        )
        .await
    }

    async fn run_session(
        ws_stream: WsStream,
        printer_id: &str,
        client_id: &str,
        username: &str,
        password: &str,
        state: Arc<RwLock<PrinterState>>,
        connected_tx: watch::Sender<bool>,
        state_changed_tx: broadcast::Sender<()>,
        mut cmd_rx: broadcast::Receiver<Command>,
        shutdown: &mut watch::Receiver<bool>,
        pending_rpcs: PendingRpcs,
    ) -> Result<(), PrinterError> {
        let (mut write, mut read) = ws_stream.split();
        let mqtt = MqttOverWs::new();
        let mut id_seq: u64 = 70;

        mqtt.send_connect(&mut write, client_id, username, password).await?;

        let code = timeout(Duration::from_secs(5), mqtt.wait_for_connack(&mut read))
            .await
            .map_err(|_| PrinterError::WebSocket("CONNACK timeout".to_string()))??;

        if code != 0 {
            return Err(PrinterError::WebSocket(format!("CONNACK rejected: code {code}")));
        }
        debug!("ws client CONNACK accepted (code=0)");

        let api_response_topic = format!("elegoo/{printer_id}/{client_id}/api_response");
        let api_status_topic = format!("elegoo/{printer_id}/api_status");
        let api_request_topic = format!("elegoo/{printer_id}/{client_id}/api_request");
        let register_response_topic = format!("elegoo/{printer_id}/{client_id}_req/register_response");
        let api_register_topic = format!("elegoo/{printer_id}/api_register");

        mqtt.subscribe(
            &mut write,
            &[
                (&register_response_topic, 1),
                (&api_response_topic, 1),
                (&api_status_topic, 0),
            ],
        )
        .await?;

        // The printer firmware requires registration before it will process commands
        // or send api_response messages. Without it, publishes are ACKed but silently ignored.
        let reg_payload = serde_json::json!({
            "client_id": client_id,
            "request_id": format!("{client_id}_req"),
        });
        if let Ok(payload) = serde_json::to_vec(&reg_payload) {
            mqtt.publish(&mut write, &api_register_topic, &payload).await?;
        }
        debug!("ws registration sent, waiting for confirmation");

        let mut registered = false;
        let mut heartbeat = interval(Duration::from_secs(10));

        loop {
            tokio::select! {
                _ = shutdown.changed() => {
                    if *shutdown.borrow() {
                        info!("ws client shutting down");
                        connected_tx.send(false).ok();
                        state_changed_tx.send(()).ok();
                        return Ok(());
                    }
                }

                _ = heartbeat.tick() => {
                    if registered {
                        if let Ok(payload) = serde_json::to_vec(&serde_json::json!({"type": "PING"})) {
                            if let Err(e) = mqtt.publish(&mut write, &api_request_topic, &payload).await {
                                warn!("[ws] heartbeat publish failed: {e}");
                            }
                        }
                    }
                }

                result = cmd_rx.recv() => {
                    match result {
                        Ok(Command { id, method, params }) => {
                            if !registered {
                                debug!("[ws-cmd] dropping method {method} - not yet registered");
                            } else {
                                let req = match params {
                                    Some(p) => serde_json::json!({"id": id, "method": method, "params": p}),
                                    None => serde_json::json!({"id": id, "method": method}),
                                };
                                if let Ok(payload) = serde_json::to_vec(&req) {
                                    debug!("[ws-cmd] sending method {method} id={id}");
                                    if let Err(e) = mqtt.publish(&mut write, &api_request_topic, &payload).await {
                                        warn!("[ws-cmd] publish method {method} failed: {e}");
                                    }
                                }
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => {
                            debug!("ws cmd channel lagged");
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            info!("ws cmd channel closed");
                            break;
                        }
                    }
                }

                msg = read.next() => {
                    match msg {
                        Some(Ok(tokio_tungstenite::tungstenite::Message::Binary(data))) => {
                            if let Some(packet) = mqtt.parse_packet(&data) {
                                match packet {
                                    WsPacket::Publish(topic, payload) => {
                                        if topic == register_response_topic {

                                            if let Ok(val) = serde_json::from_slice::<Value>(&payload) {
                                                if val.get("error").and_then(|e| e.as_str()) == Some("ok") {
                                                    registered = true;
                                                    if connected_tx.send(true).is_err() {
                                                        warn!("[ws] connected_tx watcher dropped on connect");
                                                    }
                                                    state_changed_tx.send(()).ok();
                                                    info!("ws client ready (client_id={client_id})");

                                                    let req = serde_json::json!({"id": id_seq, "method": METHOD_GET_FULL_STATUS});
                                                    id_seq += 1;
                                                    if let Ok(p) = serde_json::to_vec(&req) {
                                                        if let Err(e) = mqtt.publish(&mut write, &api_request_topic, &p).await {
                                                            warn!("[ws] initial full-status publish failed: {e}");
                                                        }
                                                    }
                                                    let req_ams = serde_json::json!({"id": id_seq, "method": METHOD_GET_AMS_INFO, "params": {}});
                                                    id_seq += 1;
                                                    if let Ok(p) = serde_json::to_vec(&req_ams) {
                                                        if let Err(e) = mqtt.publish(&mut write, &api_request_topic, &p).await {
                                                            warn!("[ws] initial ams-info publish failed: {e}");
                                                        }
                                                    }
                                                } else {
                                                    error!("ws registration rejected: {val}");
                                                    return Err(PrinterError::WebSocket("registration rejected".to_string()));
                                                }
                                            }
                                        } else {
                                            Self::handle_publish(
                                                &topic,
                                                &payload,
                                                &api_status_topic,
                                                &api_response_topic,
                                                &state,
                                                &state_changed_tx,
                                                &pending_rpcs,
                                            )
                                            .await;
                                        }
                                    }
                                    WsPacket::PubAck => trace!("ws PUBACK"),
                                    WsPacket::SubAck => trace!("ws SUBACK"),
                                    WsPacket::ConnAck(c) => trace!("ws extra CONNACK: {c}"),
                                }
                            }
                        }
                        Some(Ok(tokio_tungstenite::tungstenite::Message::Close(_))) => {
                            warn!("ws client received close frame");
                            connected_tx.send(false).ok();
                            state_changed_tx.send(()).ok();
                            return Err(PrinterError::WebSocket("connection closed".to_string()));
                        }
                        Some(Ok(_)) => {}
                        Some(Err(e)) => {
                            error!("ws client error: {e}");
                            connected_tx.send(false).ok();
                            state_changed_tx.send(()).ok();
                            return Err(PrinterError::WebSocket(e.to_string()));
                        }
                        None => {
                            warn!("ws client stream ended");
                            connected_tx.send(false).ok();
                            state_changed_tx.send(()).ok();
                            return Err(PrinterError::WebSocket("stream ended".to_string()));
                        }
                    }
                }
            }
        }

        connected_tx.send(false).ok();
        state_changed_tx.send(()).ok();
        Ok(())
    }

    async fn handle_publish(
        topic: &str,
        payload: &[u8],
        api_status_topic: &str,
        api_response_topic: &str,
        state: &Arc<RwLock<PrinterState>>,
        state_changed_tx: &broadcast::Sender<()>,
        pending_rpcs: &PendingRpcs,
    ) {
        if topic == api_status_topic {
            let Ok(value) = serde_json::from_slice::<Value>(payload) else { return };
            let msg_type = value.get("type").and_then(|t| t.as_str());
            if msg_type == Some("PING") || msg_type == Some("PONG") {
                trace!("ws heartbeat msg: {:?}", msg_type);
                return;
            }
            if value.get("method").and_then(|m| m.as_u64()) == Some(METHOD_STATUS_PUSH as u64) {
                if let Some(result) = value.get("result") {
                    state.write().await.merge_delta(result);
                    state_changed_tx.send(()).ok();
                    trace!("[ws] status delta merged");
                }
            }
        } else if topic == api_response_topic {
            let Ok(resp) = serde_json::from_slice::<RpcResponse>(payload) else { return };
            // Resolve any manager-level RPC call waiting on this id
            if resp.id > 0 {
                if let Some(tx) = pending_rpcs.lock().await.remove(&resp.id) {
                    tx.send(resp.result.data.clone()).ok();
                }
            }
            if resp.method == METHOD_GET_FULL_STATUS && resp.result.error_code == 0 {
                if let Ok(status) = serde_json::from_value::<FullStatus>(resp.result.data.clone()) {
                    state.write().await.seed(status);
                    state_changed_tx.send(()).ok();
                    info!("[ws] full status snapshot loaded");
                }
            } else if resp.method == METHOD_GET_FILE_LIST && resp.result.error_code == 0 {
                if let Some(arr) = resp.result.data.get("file_list").and_then(|v| v.as_array()) {
                    let storage = resp.result.data.get("storage_media").and_then(|v| v.as_str()).unwrap_or("local");
                    tracing::debug!("[ws] 1044 file list ({storage}): {} files", arr.len());
                    state.write().await.files = arr.clone();
                    state_changed_tx.send(()).ok();
                    info!("[ws] file list loaded: {} files", arr.len());
                }
            } else if resp.method == METHOD_GET_AMS_INFO && resp.result.error_code == 0 {
                if let Some(canvas) = resp.result.data.get("canvas_info") {
                    state.write().await.full.canvas_info = Some(canvas.clone());
                    state_changed_tx.send(()).ok();
                    info!("[ws] canvas info loaded");
                }
            }
        }
    }
}

enum WsPacket {
    Publish(String, Vec<u8>),
    ConnAck(u8),
    SubAck,
    PubAck,
}

struct MqttOverWs {
    packet_id: std::sync::atomic::AtomicU16,
}

impl MqttOverWs {
    fn new() -> Self {
        Self {
            packet_id: std::sync::atomic::AtomicU16::new(1),
        }
    }

    async fn send_connect(
        &self,
        write: &mut (impl SinkExt<tokio_tungstenite::tungstenite::Message> + Unpin),
        client_id: &str,
        username: &str,
        password: &str,
    ) -> Result<(), PrinterError> {
        let packet = encode_connect(client_id, username, password);
        write
            .send(tokio_tungstenite::tungstenite::Message::Binary(packet))
            .await
            .map_err(|_| PrinterError::WebSocket("ws CONNECT send failed".to_string()))?;
        debug!("ws sent MQTT CONNECT");
        Ok(())
    }

    async fn wait_for_connack(
        &self,
        read: &mut (impl StreamExt<
            Item = Result<
                tokio_tungstenite::tungstenite::Message,
                tokio_tungstenite::tungstenite::Error,
            >,
        > + Unpin),
    ) -> Result<u8, PrinterError> {
        while let Some(msg) = read.next().await {
            match msg {
                Ok(tokio_tungstenite::tungstenite::Message::Binary(data)) => {
                    if let Some(WsPacket::ConnAck(code)) = self.parse_packet(&data) {
                        return Ok(code);
                    }
                }
                Ok(tokio_tungstenite::tungstenite::Message::Close(_)) => {
                    return Err(PrinterError::WebSocket("closed waiting for CONNACK".to_string()));
                }
                Err(e) => {
                    return Err(PrinterError::WebSocket(format!("read error: {e}")));
                }
                _ => {}
            }
        }
        Err(PrinterError::WebSocket("stream ended waiting for CONNACK".to_string()))
    }

    async fn subscribe(
        &self,
        write: &mut (impl SinkExt<tokio_tungstenite::tungstenite::Message> + Unpin),
        topics: &[(&str, u8)],
    ) -> Result<(), PrinterError> {
        let pid = self.packet_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let packet = encode_subscribe(pid, topics);
        write
            .send(tokio_tungstenite::tungstenite::Message::Binary(packet))
            .await
            .map_err(|_| PrinterError::WebSocket("ws SUBSCRIBE send failed".to_string()))?;
        Ok(())
    }

    async fn publish(
        &self,
        write: &mut (impl SinkExt<tokio_tungstenite::tungstenite::Message> + Unpin),
        topic: &str,
        payload: &[u8],
    ) -> Result<(), PrinterError> {
        let pid = self.packet_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let packet = encode_publish(1, pid, topic, payload);
        write
            .send(tokio_tungstenite::tungstenite::Message::Binary(packet))
            .await
            .map_err(|_| PrinterError::WebSocket("ws PUBLISH send failed".to_string()))?;
        Ok(())
    }

    fn parse_packet(&self, data: &[u8]) -> Option<WsPacket> {
        if data.is_empty() {
            return None;
        }
        let packet_type = (data[0] >> 4) & 0x0F;
        match packet_type {
            2 => {
                // CONNACK: fixed header(1) + remaining_len(1) + session_present(1) + return_code(1)
                if data.len() < 4 {
                    warn!("[ws] CONNACK too short ({} bytes)", data.len());
                    return None;
                }
                Some(WsPacket::ConnAck(data[3]))
            }
            3 => {
                // PUBLISH: fixed header(1) + remaining_len(1-4) + topic_len(2) + topic + [packet_id(2)] + payload
                let mut pos = 1usize;
                let (remaining, br) = decode_remaining_len(data.get(pos..)?)?;
                pos += br;

                // Sanity check: remaining length must not exceed actual data
                if pos.saturating_add(remaining) > data.len() {
                    warn!("[ws] PUBLISH remaining_len={remaining} exceeds packet size={}", data.len());
                    return None;
                }

                if pos + 2 > data.len() {
                    warn!("[ws] PUBLISH too short for topic length field");
                    return None;
                }
                let topic_len = u16::from_be_bytes([data[pos], data[pos + 1]]) as usize;
                pos += 2;

                if pos + topic_len > data.len() {
                    warn!("[ws] PUBLISH topic_len={topic_len} exceeds packet size={}", data.len());
                    return None;
                }
                let topic = String::from_utf8_lossy(&data[pos..pos + topic_len]).to_string();
                pos += topic_len;

                // QoS 1/2: skip the 2-byte packet identifier that follows the topic
                let qos = (data[0] >> 1) & 0x03;
                if qos > 0 {
                    if pos + 2 > data.len() {
                        warn!("[ws] PUBLISH QoS={qos} but packet too short for packet-id");
                        return None;
                    }
                    pos += 2;
                }

                Some(WsPacket::Publish(topic, data[pos..].to_vec()))
            }
            4 => Some(WsPacket::PubAck),
            9 => Some(WsPacket::SubAck),
            other => {
                trace!("[ws] unhandled packet type {other:#x}");
                None
            }
        }
    }
}

fn encode_connect(client_id: &str, username: &str, password: &str) -> Vec<u8> {
    let cid = client_id.as_bytes();
    let uname = username.as_bytes();
    let pwd = password.as_bytes();
    // variable header (10) + length-prefixed client_id, username, password
    let remaining = 10 + 2 + cid.len() + 2 + uname.len() + 2 + pwd.len();

    let mut buf = vec![0x10]; // CONNECT fixed header
    buf.extend_from_slice(&encode_remaining_len(remaining));
    buf.extend_from_slice(&[0x00, 0x04]);
    buf.extend_from_slice(b"MQTT");
    buf.push(0x04); // protocol level 3.1.1
    buf.push(0xC2); // connect flags: username + password + clean session
    buf.extend_from_slice(&[0x00, 0x3C]); // keep-alive: 60s
    buf.extend_from_slice(&(cid.len() as u16).to_be_bytes());
    buf.extend_from_slice(cid);
    buf.extend_from_slice(&(uname.len() as u16).to_be_bytes());
    buf.extend_from_slice(uname);
    buf.extend_from_slice(&(pwd.len() as u16).to_be_bytes());
    buf.extend_from_slice(pwd);
    buf
}

fn encode_subscribe(packet_id: u16, topics: &[(&str, u8)]) -> Vec<u8> {
    let remaining = 2 + topics.iter().map(|(t, _)| 2 + t.len() + 1).sum::<usize>();
    let mut buf = vec![0x82]; // SUBSCRIBE fixed header
    buf.extend_from_slice(&encode_remaining_len(remaining));
    buf.extend_from_slice(&packet_id.to_be_bytes());
    for (topic, qos) in topics {
        buf.extend_from_slice(&(topic.len() as u16).to_be_bytes());
        buf.extend_from_slice(topic.as_bytes());
        buf.push(*qos);
    }
    buf
}

fn encode_publish(qos: u8, packet_id: u16, topic: &str, payload: &[u8]) -> Vec<u8> {
    let topic_bytes = topic.as_bytes();
    let has_pid = qos > 0;
    let remaining = 2 + topic_bytes.len() + if has_pid { 2 } else { 0 } + payload.len();

    let mut buf = vec![0x30 | ((qos & 0x03) << 1)];
    buf.extend_from_slice(&encode_remaining_len(remaining));
    buf.extend_from_slice(&(topic_bytes.len() as u16).to_be_bytes());
    buf.extend_from_slice(topic_bytes);
    if has_pid {
        buf.extend_from_slice(&packet_id.to_be_bytes());
    }
    buf.extend_from_slice(payload);
    buf
}

fn encode_remaining_len(mut len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(4);
    loop {
        let mut byte = (len % 128) as u8;
        len /= 128;
        if len > 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if len == 0 {
            break;
        }
    }
    out
}

fn decode_remaining_len(data: &[u8]) -> Option<(usize, usize)> {
    let mut multiplier = 1usize;
    let mut value = 0usize;
    for (i, &byte) in data.iter().enumerate() {
        value += (byte & 0x7F) as usize * multiplier;
        if byte & 0x80 == 0 {
            return Some((value, i + 1));
        }
        multiplier *= 128;
        if i >= 3 {
            return None;
        }
    }
    None
}
