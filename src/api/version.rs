use axum::extract::State;
use axum::Json;

use super::router::AppState;

pub async fn get_version(State(s): State<AppState>) -> Json<serde_json::Value> {
    let status = s.update_checker.status.read().await;
    Json(serde_json::json!({
        "current_sha": status.current_sha,
        "latest_sha": status.latest_sha,
        "up_to_date": status.up_to_date,
    }))
}
