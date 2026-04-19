use std::time::Duration;

use reqwest::Client;
use tracing::{debug, info, warn};

use crate::config::ExcludeZone;
use crate::error::DetectionError;

/// A single spaghetti detection from the Obico ML API.
/// Coordinates are normalized 0.0-1.0 relative to the image dimensions.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Detection {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
    pub confidence: f64,
}

pub struct ObicoResult {
    pub score: f64,
    pub detections: Vec<Detection>,
}

pub struct ObicoClient {
    client: Client,
    base_url: String,
}

impl ObicoClient {
    pub fn new(base_url: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .connect_timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_default();
        Self {
            base_url: normalize_base_url(base_url),
            client,
        }
    }

    /// Send frame to Obico and return score plus detections.
    /// Detections inside exclude zones are filtered before scoring.
    pub async fn analyze_snapshot(
        &self,
        obico_img_url: &str,
        frame: &[u8],
        exclude_zones: &[ExcludeZone],
    ) -> Result<ObicoResult, DetectionError> {
        let img_dims = read_jpeg_dims(frame);
        let url = format!("{}/p/", self.base_url);
        info!("[obico] GET {url}?img={obico_img_url} (frame_dims={img_dims:?})");

        let response = self
            .client
            .get(&url)
            .query(&[("img", obico_img_url)])
            .send()
            .await
            .map_err(|e| DetectionError::ObicoFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(DetectionError::ObicoFailed(format!(
                "Obico returned {}",
                response.status()
            )));
        }

        let body = response
            .text()
            .await
            .map_err(|e| DetectionError::ObicoFailed(e.to_string()))?;

        debug!("[obico] raw response: {body}");
        let result = parse_obico_response(&body, exclude_zones, img_dims)?;
        if result.score == 0.0 && result.detections.is_empty() {
            debug!("[obico] score=0 detections=[] - no failures detected in this frame");
        }
        Ok(result)
    }
}

/// Save a JPEG frame when a non-zero detection score is observed.
/// Stored as `snapshots/detection_{timestamp}_{score_pct}.jpg`.
/// Also writes a sidecar `.json` file with detection box data.
/// Returns the saved path on success.
pub fn save_detection_snapshot(jpeg: &[u8], score: f64, detections: &[Detection]) -> Option<std::path::PathBuf> {
    let snap_dir = std::path::Path::new("snapshots");
    let _ = std::fs::create_dir_all(snap_dir);
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let score_pct = (score * 100.0).round() as u32;
    let path = snap_dir.join(format!("detection_{ts}_{score_pct}.jpg"));
    match std::fs::write(&path, jpeg) {
        Ok(_) => {
            info!("[obico] snapshot saved: {} ({} bytes, score={:.2})", path.display(), jpeg.len(), score);
            if !detections.is_empty() {
                let json_path = path.with_extension("json");
                if let Ok(f) = std::fs::File::create(&json_path) {
                    let _ = serde_json::to_writer(f, detections);
                }
            }
            prune_snapshots(snap_dir, 100);
            Some(path)
        }
        Err(e) => {
            warn!("[obico] could not write snapshot: {e}");
            None
        }
    }
}

/// Keep the snapshots directory bounded to `max_files` jpg entries.
/// Deletes the oldest jpg files and their json sidecars when over the limit.
fn prune_snapshots(dir: &std::path::Path, max_files: usize) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut files: Vec<(std::time::SystemTime, std::path::PathBuf)> = entries
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            if path.extension().and_then(|x| x.to_str()) != Some("jpg") { return None; }
            let mtime = e.metadata().ok()?.modified().ok()?;
            Some((mtime, path))
        })
        .collect();

    if files.len() <= max_files {
        return;
    }

    files.sort_by_key(|(t, _)| *t);
    for (_, path) in files.iter().take(files.len() - max_files) {
        if let Err(e) = std::fs::remove_file(path) {
            warn!("[obico] failed to prune old snapshot {}: {e}", path.display());
        } else {
            let _ = std::fs::remove_file(path.with_extension("json"));
        }
    }
}

/// Trim trailing /p variants so URL ends with /p/.
fn normalize_base_url(raw: &str) -> String {
    let mut s = raw.trim().trim_end_matches('/').to_string();
    if s.ends_with("/p") {
        s.truncate(s.len() - 2);
    }
    s.trim_end_matches('/').to_string()
}

/// Scan JPEG bytes for an SOF marker (0xFF 0xC0 or 0xFF 0xC2) to read (width, height).
pub fn read_jpeg_dims(jpeg: &[u8]) -> Option<(f64, f64)> {
    if jpeg.len() < 4 || jpeg[0] != 0xFF || jpeg[1] != 0xD8 {
        return None;
    }
    let mut i = 2usize;
    while i + 3 < jpeg.len() {
        if jpeg[i] != 0xFF {
            break;
        }
        let marker = jpeg[i + 1];
        if marker == 0xD9 {
            break; // EOI
        }
        // SOF0 (baseline) or SOF2 (progressive)
        if (marker == 0xC0 || marker == 0xC2) && i + 8 < jpeg.len() {
            let h = ((jpeg[i + 5] as u32) << 8) | (jpeg[i + 6] as u32);
            let w = ((jpeg[i + 7] as u32) << 8) | (jpeg[i + 8] as u32);
            if w > 0 && h > 0 {
                return Some((w as f64, h as f64));
            }
        }
        // skip this segment: 2 bytes marker + 2 bytes length field
        let seg_len = ((jpeg[i + 2] as usize) << 8) | (jpeg[i + 3] as usize);
        i += 2 + seg_len;
    }
    None
}

/// Parse Obico response and normalize boxes to 0..1.
/// Supports COCO detections and legacy fallback formats.
fn parse_obico_response(
    body: &str,
    exclude_zones: &[ExcludeZone],
    img_dims: Option<(f64, f64)>,
) -> Result<ObicoResult, DetectionError> {
    let value: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| DetectionError::ObicoFailed(format!("invalid JSON: {e}")))?;

    let all_detections: Vec<Detection> = value
        .get("detections")
        .and_then(|d| d.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|det| parse_single_detection(det, img_dims))
                .collect()
        })
        .unwrap_or_default();

    let filtered: Vec<Detection> = all_detections
        .iter()
        .filter(|d| {
            !exclude_zones
                .iter()
                .any(|z| z.contains_center(d.x1, d.y1, d.x2, d.y2))
        })
        .cloned()
        .collect();

    let score = if !filtered.is_empty() {
        filtered.iter().map(|d| d.confidence).fold(0.0_f64, f64::max)
    } else if !all_detections.is_empty() && all_detections_excluded(&all_detections, exclude_zones) {
        0.0
    } else {
        // Fallback: some Obico variants include a top-level score field.
        value
            .get("fail")
            .or_else(|| value.get("score"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0)
    };

    debug!(
        "[obico] score={score:.4} raw_detections={} filtered={}",
        all_detections.len(),
        filtered.len()
    );
    Ok(ObicoResult { score, detections: filtered })
}

/// Parse one detection entry (COCO/object/legacy array).
fn parse_single_detection(
    det: &serde_json::Value,
    img_dims: Option<(f64, f64)>,
) -> Option<Detection> {
    if let Some(arr) = det.as_array() {
        // Format 1: ["class_label", confidence, [cx, cy, w, h]]
        if arr.len() == 3 {
            if let (Some(conf), Some(bbox)) = (arr[1].as_f64(), arr[2].as_array()) {
                if bbox.len() == 4 {
                    let nums: Vec<f64> = bbox.iter().filter_map(|v| v.as_f64()).collect();
                    if nums.len() == 4 {
                        let (cx, cy, pw, ph) = (nums[0], nums[1], nums[2], nums[3]);
                        let (img_w, img_h) = img_dims.unwrap_or((1.0, 1.0));
                        return Some(Detection {
                            x1: (cx - pw / 2.0) / img_w,
                            y1: (cy - ph / 2.0) / img_h,
                            x2: (cx + pw / 2.0) / img_w,
                            y2: (cy + ph / 2.0) / img_h,
                            confidence: conf,
                        });
                    }
                }
            }
        }
        // Format 3: [x1, y1, x2, y2, confidence] already normalized
        if arr.len() >= 5 {
            let nums: Vec<f64> = arr.iter().filter_map(|v| v.as_f64()).collect();
            if nums.len() >= 5 {
                return Some(Detection {
                    x1: nums[0],
                    y1: nums[1],
                    x2: nums[2],
                    y2: nums[3],
                    confidence: nums[4],
                });
            }
        }
        None
    } else if det.is_object() {
        // Format 2: object with x1/y1/x2/y2/confidence
        let x1 = det.get("x1").and_then(|v| v.as_f64())?;
        let y1 = det.get("y1").and_then(|v| v.as_f64())?;
        let x2 = det.get("x2").and_then(|v| v.as_f64())?;
        let y2 = det.get("y2").and_then(|v| v.as_f64())?;
        let conf = det.get("confidence")
            .or_else(|| det.get("score"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        Some(Detection { x1, y1, x2, y2, confidence: conf })
    } else {
        None
    }
}

/// Returns true when all detections were excluded by zone filtering.
fn all_detections_excluded(detections: &[Detection], exclude_zones: &[ExcludeZone]) -> bool {
    !exclude_zones.is_empty()
        && !detections.is_empty()
        && detections.iter().all(|d| {
            exclude_zones
                .iter()
                .any(|z| z.contains_center(d.x1, d.y1, d.x2, d.y2))
        })
}
