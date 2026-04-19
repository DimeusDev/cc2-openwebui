use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Shared JPEG frame buffer. None until first frame arrives.
pub type FrameBuffer = Arc<RwLock<Option<Vec<u8>>>>;

/// Spawns the background task that owns the single MJPEG connection.
pub fn spawn_frame_grabber(camera_ip: String, buffer: FrameBuffer) {
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
                                    *buffer.write().await = Some(frame);
                                }

                                // drop old data if no good frame
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

                    // Reset backoff if the stream was healthy for a while.
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
}

/// extract first jpeg from `buf`,
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
