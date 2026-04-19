use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{broadcast, Mutex, RwLock};
use tokio::time::interval;
use tracing::{debug, info, warn};

use super::obico::ObicoClient;
use crate::camera::FrameBuffer;
use crate::config::DetectionConfig;
use crate::printer::manager::PrinterManager;
use crate::printer::state::{DetectionPoint, EventKind, PrintState, PrinterState};

pub struct DetectionEngine {
    obico: ObicoClient,
    config: DetectionConfig,
    server_port: u16,
    frame_buffer: FrameBuffer,
    scores: std::collections::VecDeque<f64>,
    consecutive_notify: u32,
    consecutive_pause: u32,
    current_score: f64,
    enabled: bool,
}

impl DetectionEngine {
    pub fn new(
        config: DetectionConfig,
        server_port: u16,
        frame_buffer: FrameBuffer,
    ) -> Self {
        Self {
            obico: ObicoClient::new(&config.obico_url),
            config,
            server_port,
            frame_buffer,
            scores: std::collections::VecDeque::with_capacity(10),
            consecutive_notify: 0,
            consecutive_pause: 0,
            current_score: 0.0,
            enabled: true,
        }
    }

    pub async fn run(
        mut self,
        state: Arc<RwLock<PrinterState>>,
        manager: Arc<Mutex<PrinterManager>>,
        mut enabled_rx: tokio::sync::watch::Receiver<bool>,
        mut config_rx: tokio::sync::watch::Receiver<DetectionConfig>,
        mut shutdown: tokio::sync::watch::Receiver<bool>,
        state_changed_tx: broadcast::Sender<()>,
    ) {
        let mut tick = interval(Duration::from_secs(self.config.interval_secs as u64));

        loop {
            tokio::select! {
                _ = shutdown.changed() => {
                    if *shutdown.borrow() {
                        info!("[detection] shutting down");
                        return;
                    }
                }
                _ = enabled_rx.changed() => {
                    self.enabled = *enabled_rx.borrow();
                    info!("[detection] enabled={}", self.enabled);
                }
                _ = config_rx.changed() => {
                    let new_config = config_rx.borrow().clone();
                    if new_config.obico_url != self.config.obico_url {
                        self.obico = ObicoClient::new(&new_config.obico_url);
                        info!("[detection] Obico client reinitialized (url={})", new_config.obico_url);
                    }
                    if new_config.interval_secs != self.config.interval_secs {
                        tick = interval(Duration::from_secs(new_config.interval_secs as u64));
                        info!("[detection] poll interval updated to {}s", new_config.interval_secs);
                    }
                    self.config = new_config;
                    info!("[detection] config updated");
                }
                _ = tick.tick() => {
                    if !self.enabled {
                        debug!("[detection] skipping - detection disabled");
                        continue;
                    }

                    let (print_state, print_filename) = {
                        let s = state.read().await;
                        let ps = s.print_state();
                        let fname = s.full.print_status.filename.clone();
                        (ps, if fname.is_empty() { None } else { Some(fname) })
                    };
                    if !matches!(print_state, PrintState::Printing) {
                        debug!("[detection] skipping - printer not printing ({:?})", print_state);
                        self.consecutive_notify = 0;
                        self.consecutive_pause = 0;
                        continue;
                    }

                    let frame = self.frame_buffer.read().await.clone();
                    let Some(jpeg) = frame else {
                        debug!("[detection] skipping - no frame available yet");
                        continue;
                    };

                    let proxy_url = format!("http://127.0.0.1:{}/api/camera/snapshot", self.server_port);
                    let exclude_zones = self.config.exclude_zones.clone();

                    match self.obico.analyze_snapshot(&proxy_url, &jpeg, &exclude_zones).await {
                        Ok(result) => {
                            let score = result.score;
                            self.current_score = score;
                            self.push_score(score);

                            let rolling_avg = self.rolling_average();
                            debug!(
                                "[detection] score={:.4} rolling={:.4} detections={} notify_consec={} pause_consec={}",
                                score,
                                rolling_avg,
                                result.detections.len(),
                                self.consecutive_notify,
                                self.consecutive_pause,
                            );

                            let now = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs();

                            let detections = result.detections;

                            let snapshot_name = if score > 0.0 {
                                super::obico::save_detection_snapshot(&jpeg, score, &detections)
                                    .and_then(|p| {
                                        p.file_name()
                                            .map(|n| n.to_string_lossy().to_string())
                                    })
                            } else {
                                None
                            };

                            {
                                let mut s = state.write().await;
                                s.detection_score = score;
                                s.detection_history.push_back(DetectionPoint {
                                    ts: now,
                                    score,
                                    snapshot: snapshot_name.clone(),
                                    print_filename: print_filename.clone(),
                                    boxes: detections.clone(),
                                });
                                if s.detection_history.len() > 30 {
                                    s.detection_history.pop_front();
                                }
                                s.latest_detections = detections;
                                s.latest_detection_ts = now;

                                if score > 0.0 {
                                    s.add_event_with_snapshot(
                                        EventKind::DetectionLogged,
                                        format!("Detection score {:.0}%", score * 100.0),
                                        snapshot_name,
                                    );
                                }
                            }
                            state_changed_tx.send(()).ok();

                            // Notify threshold check
                            if rolling_avg >= self.config.notify_threshold {
                                self.consecutive_notify += 1;
                                if self.consecutive_notify == self.config.confirmation_frames {
                                    warn!(
                                        "[detection] notify threshold confirmed: rolling={rolling_avg:.4}",
                                    );
                                    state.write().await.add_event(
                                        EventKind::FailureNotifyThreshold,
                                        format!("Failure risk detected (score: {:.0}%)", rolling_avg * 100.0),
                                    );
                                }
                            } else {
                                self.consecutive_notify = 0;
                            }

                            // Pause threshold check
                            if rolling_avg >= self.config.pause_threshold {
                                self.consecutive_pause += 1;
                                if self.consecutive_pause == self.config.confirmation_frames {
                                    warn!(
                                        "[detection] pause threshold confirmed: rolling={rolling_avg:.4}",
                                    );
                                    state.write().await.add_event(
                                        EventKind::FailurePauseThreshold,
                                        format!("Print failure confirmed (score: {:.0}%), pausing", rolling_avg * 100.0),
                                    );
                                    state.write().await.add_event(
                                        EventKind::AutoPaused,
                                        "Print auto-paused by detection engine".to_string(),
                                    );
                                    if let Err(e) = manager.lock().await.pause().await {
                                        warn!("[detection] auto-pause failed: {e}");
                                    }
                                }
                            } else {
                                self.consecutive_pause = 0;
                            }
                        }
                        Err(e) => {
                            warn!("[detection] analysis failed: {e}");
                        }
                    }
                }
            }
        }
    }

    fn push_score(&mut self, score: f64) {
        self.scores.push_back(score);
        if self.scores.len() > 10 {
            self.scores.pop_front();
        }
    }

    fn rolling_average(&self) -> f64 {
        if self.scores.is_empty() {
            return 0.0;
        }
        self.scores.iter().sum::<f64>() / self.scores.len() as f64
    }
}
