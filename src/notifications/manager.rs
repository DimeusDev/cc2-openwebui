use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{broadcast, RwLock};
use tracing::warn;

use crate::config::{AppConfig, DestinationKind, EventToggles, NotificationDestination};
use crate::printer::state::{EventKind, PrinterEvent, PrinterState};

use super::{discord, ntfy, payload};

const COOLDOWN_SECS: u64 = 120;

pub struct NotificationManager {
    state: Arc<RwLock<PrinterState>>,
    config: Arc<RwLock<AppConfig>>,
    last_processed: usize,
    cooldowns: HashMap<String, Instant>,
}

impl NotificationManager {
    pub fn new(state: Arc<RwLock<PrinterState>>, config: Arc<RwLock<AppConfig>>) -> Self {
        Self {
            state,
            config,
            last_processed: 0,
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
            let events = &state.events;

            let start = if self.last_processed <= events.len() {
                self.last_processed
            } else {
                0
            };

            let new: Vec<PrinterEvent> = events[start..].to_vec();
            self.last_processed = events.len();

            let config = self.config.read().await;
            (new, config.notifications.destinations.clone())
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
        _ => false,
    }
}

async fn dispatch(dest: &NotificationDestination, title: &str, body: &str, color: u32) {
    match dest.kind {
        DestinationKind::Ntfy => {
            if let Err(e) = ntfy::send(dest, title, body).await {
                warn!("[notifications] ntfy '{}' failed: {e}", dest.label);
            }
        }
        DestinationKind::Discord => {
            if let Err(e) = discord::send(dest, title, body, color).await {
                warn!("[notifications] discord '{}' failed: {e}", dest.label);
            }
        }
        DestinationKind::Webhook => {
        }
    }
}
