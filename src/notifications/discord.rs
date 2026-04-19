use std::time::Duration;

use reqwest::Client;
use serde_json::json;
use tracing::warn;

use crate::config::NotificationDestination;
use crate::error::NotificationError;

pub async fn send_test(dest: &NotificationDestination) -> Result<(), NotificationError> {
    send(dest, "CC2 Monitor", "Test notification - Discord webhook is working", 0x3498db).await
}

pub async fn send(
    dest: &NotificationDestination,
    title: &str,
    body: &str,
    color: u32,
) -> Result<(), NotificationError> {
    let url = match dest.discord_webhook_url.as_deref() {
        Some(u) if !u.is_empty() => u.to_string(),
        _ => return Err(NotificationError::DiscordFailed("webhook URL is not configured".to_string())),
    };

    let payload = json!({
        "embeds": [{
            "title": title,
            "description": body,
            "color": color,
        }]
    });

    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap_or_default();

    let res = client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| NotificationError::DiscordFailed(e.to_string()))?;

    if !res.status().is_success() {
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        warn!("discord returned {status}: {text}");
        return Err(NotificationError::DiscordFailed(format!("server returned {status}")));
    }

    Ok(())
}
