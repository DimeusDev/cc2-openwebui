use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use tokio::sync::RwLock;
use tracing::{info, warn};

const CURRENT_SHA: &str = env!("GIT_HASH");
const REPO: &str = "DimeusDev/cc2-openwebui";
const CHECK_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Clone, Serialize)]
pub struct UpdateStatus {
    pub current_sha: String,
    pub latest_sha: Option<String>,
    pub up_to_date: bool,
}

pub struct UpdateChecker {
    pub status: Arc<RwLock<UpdateStatus>>,
    client: reqwest::Client,
}

impl UpdateChecker {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            status: Arc::new(RwLock::new(UpdateStatus {
                current_sha: CURRENT_SHA.to_string(),
                latest_sha: None,
                up_to_date: true,
            })),
            client: reqwest::Client::new(),
        })
    }

    async fn check(&self) {
        let url = format!("https://api.github.com/repos/{REPO}/commits/main");
        let result = self
            .client
            .get(&url)
            .header("Accept", "application/vnd.github.sha")
            .header("User-Agent", "cc2-openwebui-update-check")
            .timeout(Duration::from_secs(15))
            .send()
            .await;

        match result {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(body) = resp.text().await {
                    let latest = body.trim().to_string();
                    // treat unknown build as always up to date so we never false-alert
                    let up_to_date = CURRENT_SHA == "unknown" || latest == CURRENT_SHA;
                    let mut s = self.status.write().await;
                    s.latest_sha = Some(latest.clone());
                    s.up_to_date = up_to_date;
                    info!(current = CURRENT_SHA, latest = %latest, up_to_date, "update check complete");
                }
            }
            Ok(resp) => warn!("update check: GitHub returned {}", resp.status()),
            Err(e) => warn!("update check: {e}"),
        }
    }

    pub fn start(self: Arc<Self>) {
        tokio::spawn(async move {
            loop {
                self.check().await;
                tokio::time::sleep(CHECK_INTERVAL).await;
            }
        });
    }
}
