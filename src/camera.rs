use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bytes::Bytes;
use futures::StreamExt;
use tokio::sync::{broadcast, watch, RwLock};
use tracing::{debug, info, warn};

/// jpeg frame buffer (used by detection engine)
pub type FrameBuffer = Arc<RwLock<Option<Vec<u8>>>>;
pub type FrameBroadcast = Arc<broadcast::Sender<Bytes>>;
pub type CameraConnectedRx = watch::Receiver<bool>;

pub struct CameraStatus {
    pub connected: AtomicBool,
    pub frame_count: AtomicU64,
    /// unix ms, -1 = never
    pub last_frame_ms: AtomicI64,
}

impl CameraStatus {
    fn new() -> Self {
        Self {
            connected: AtomicBool::new(false),
            frame_count: AtomicU64::new(0),
            last_frame_ms: AtomicI64::new(-1),
        }
    }
}

impl Default for CameraStatus {
    fn default() -> Self { Self::new() }
}

pub fn spawn_frame_grabber(
    camera_ip: String,
    buffer: FrameBuffer,
) -> (Arc<CameraStatus>, FrameBroadcast, CameraConnectedRx) {
    let status = Arc::new(CameraStatus::new());
    let (broadcast_tx, _) = broadcast::channel(8);
    let broadcast_tx = Arc::new(broadcast_tx);
    let (connected_tx, connected_rx) = watch::channel(false);

    let status_clone = status.clone();
    let broadcast_clone = broadcast_tx.clone();

    tokio::spawn(async move {
        let mut backoff = 2u64;

        loop {
            let url = format!("http://{}:8080/?action=stream", camera_ip);
            info!("[grabber] connecting to {url}");

            let client = match reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(5))
                .build()
            {
                Ok(c) => c,
                Err(e) => {
                    warn!("[grabber] failed to build client: {e}");
                    tokio::time::sleep(Duration::from_secs(backoff)).await;
                    backoff = (backoff * 2).min(30);
                    continue;
                }
            };

            match client.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    info!("[grabber] stream connected");
                    status_clone.connected.store(true, Ordering::Relaxed);
                    let _ = connected_tx.send(true);

                    let stream_start = std::time::Instant::now();
                    let mut stream = resp.bytes_stream();
                    let mut buf: Vec<u8> = Vec::with_capacity(128 * 1024);
                    let mut frames: u64 = 0;

                    while let Some(chunk) = stream.next().await {
                        match chunk {
                            Ok(data) => {
                                buf.extend_from_slice(&data);

                                while let Some(frame) = try_extract_frame(&mut buf) {
                                    frames += 1;
                                    if frames == 1 || frames % 60 == 0 {
                                        debug!("[grabber] frame #{frames} ({} bytes)", frame.len());
                                    }

                                    let now_ms = SystemTime::now()
                                        .duration_since(UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_millis() as i64;
                                    status_clone.frame_count.fetch_add(1, Ordering::Relaxed);
                                    status_clone.last_frame_ms.store(now_ms, Ordering::Relaxed);

                                    let bframe = Bytes::copy_from_slice(&frame);
                                    let _ = broadcast_clone.send(bframe);

                                    *buffer.write().await = Some(frame);
                                }

                                if buf.len() > 4 * 1024 * 1024 {
                                    warn!("[grabber] buffer flushing");
                                    buf.clear();
                                }
                            }
                            Err(e) => {
                                warn!("[grabber] stream error: {e}");
                                break;
                            }
                        }
                    }

                    status_clone.connected.store(false, Ordering::Relaxed);
                    let _ = connected_tx.send(false);

                    if stream_start.elapsed().as_secs() >= 10 {
                        backoff = 2;
                    }

                    warn!("[grabber] stream ended after {frames} frames, reconnecting in {backoff}s");
                }
                Ok(resp) => {
                    warn!("[grabber] camera returned {}, retrying in {backoff}s", resp.status());
                }
                Err(e) => {
                    warn!("[grabber] camera unreachable: {e}, retrying in {backoff}s");
                }
            }

            tokio::time::sleep(Duration::from_secs(backoff)).await;
            backoff = (backoff * 2).min(30);
        }
    });

    (status, broadcast_tx, connected_rx)
}

fn try_extract_frame(buf: &mut Vec<u8>) -> Option<Vec<u8>> {
    let start = find_seq(buf, &[0xFF, 0xD8])?;
    let rel_end = find_seq(&buf[start..], &[0xFF, 0xD9])?;
    let end = start + rel_end + 2;
    let frame = buf[start..end].to_vec();
    buf.drain(..end);
    Some(frame)
}

fn find_seq(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}
