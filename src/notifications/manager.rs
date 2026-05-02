use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn};

use crate::config::{AppConfig, DestinationKind, EventToggles, NotificationDestination};
use crate::printer::state::{EventKind, PrinterEvent, PrinterState};

use super::{discord, ntfy, payload, webhook};

const COOLDOWN_SECS: u64 = 120;

pub struct NotificationManager {
    state: Arc<RwLock<PrinterState>>,
    config: Arc<RwLock<AppConfig>>,
    /// last events_total
    last_processed_total: u64,
    cooldowns: HashMap<String, Instant>,
}

impl NotificationManager {
    pub fn new(state: Arc<RwLock<PrinterState>>, config: Arc<RwLock<AppConfig>>) -> Self {
        // seed current total
        let last_processed_total = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                state.read().await.events_total
            })
        });
        Self {
            state,
            config,
            last_processed_total,
            cooldowns: HashMap::new(),
        }
    }

    pub async fn run(mut self, mut state_changed_rx: broadcast::Receiver<()>) {
        loop {
            match state_changed_rx.recv().await {
                Ok(()) => self.process_new_events().await,
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!("[notifications] missed {n} state updates");
                    self.process_new_events().await;
                }
                Err(broadcast::error::RecvError::Closed) => return,
            }
        }
    }

    async fn process_new_events(&mut self) {
        let (new_events, destinations) = {
            let state = self.state.read().await;
            let events_total = state.events_total;
            let events = &state.events;

            // new events count
            let unprocessed = (events_total.saturating_sub(self.last_processed_total)) as usize;
            // cap by buffer
            let to_take = unprocessed.min(events.len());
            let new: Vec<PrinterEvent> = events[events.len() - to_take..].to_vec();

            self.last_processed_total = events_total;

            let destinations = self.config.read().await.notifications.destinations.clone();
            (new, destinations)
        };

        for event in &new_events {
            for dest in &destinations {
                if !dest.enabled {
                    continue;
                }
                if !event_matches_toggles(&event.kind, &dest.toggles) {
                    continue;
                }

                let key = format!("{}:{}", dest.id, event_kind_label(&event.kind));
                if let Some(last) = self.cooldowns.get(&key) {
                    if last.elapsed() < Duration::from_secs(COOLDOWN_SECS) {
                        continue;
                    }
                }
                self.cooldowns.insert(key, Instant::now());

                let p = payload::format_event(event);
                dispatch(dest, &p.title, &p.body, p.color).await;
            }
        }
    }
}

fn event_kind_label(kind: &EventKind) -> &'static str {
    match kind {
        EventKind::PrintStarted => "print_started",
        EventKind::PrintFinished => "print_finished",
        EventKind::PrintPaused => "print_paused",
        EventKind::FailureNotifyThreshold => "failure_notify",
        EventKind::FailurePauseThreshold => "failure_pause",
        EventKind::AutoPaused => "auto_paused",
        EventKind::CameraLost => "camera_lost",
        EventKind::CameraRestored => "camera_restored",
        _ => "other",
    }
}

fn event_matches_toggles(kind: &EventKind, t: &EventToggles) -> bool {
    match kind {
        EventKind::PrintStarted => t.print_started,
        EventKind::PrintFinished => t.print_finished,
        EventKind::PrintPaused => t.print_paused,
        EventKind::FailureNotifyThreshold => t.failure_notify,
        EventKind::FailurePauseThreshold => t.failure_pause,
        EventKind::AutoPaused => t.auto_paused,
        EventKind::CameraLost => t.camera_lost,
        EventKind::CameraRestored => t.camera_restored,
        _ => false,
    }
}

async fn dispatch(dest: &NotificationDestination, title: &str, body: &str, color: u32) {
    match dest.kind {
        DestinationKind::Ntfy => match ntfy::send(dest, title, body).await {
            Ok(()) => info!("[notifications] ntfy '{}' sent: {title}", dest.label),
            Err(e) => warn!("[notifications] ntfy '{}' failed: {e}", dest.label),
        },
        DestinationKind::Discord => match discord::send(dest, title, body, color).await {
            Ok(()) => info!("[notifications] discord '{}' sent: {title}", dest.label),
            Err(e) => warn!("[notifications] discord '{}' failed: {e}", dest.label),
        },
        DestinationKind::Webhook => match webhook::send(dest, title, body).await {
            Ok(()) => info!("[notifications] webhook '{}' sent: {title}", dest.label),
            Err(e) => warn!("[notifications] webhook '{}' failed: {e}", dest.label),
        },
    }
}
