use axum::response::sse::{Event, Sse};
use axum::Json;
use serde::Deserialize;
use serde_json::Value;
use std::convert::Infallible;
use std::time::Duration;

use crate::docker;
use crate::error::AppError;

pub async fn get_status() -> Json<Value> {
    let status = docker::status().await;
    Json(serde_json::json!({ "status": status }))
}

pub async fn start_container() -> Result<Json<Value>, AppError> {
    docker::start().await.map_err(AppError::Validation)?;
    let ready = docker::wait_for_ready(60).await;
    if !ready {
        return Err(AppError::Validation(
            "Container started but ML API did not become reachable within 60s".to_string(),
        ));
    }
    Ok(Json(serde_json::json!({
        "success": true,
        "url": format!("http://127.0.0.1:{}/p/", docker::OBICO_PORT),
    })))
}

pub async fn start_container_stream(
) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    let (tx, rx) = tokio::sync::mpsc::channel::<(String, String)>(64);

    tokio::spawn(async move {
        match docker::status().await {
            docker::ObicoStatus::Running => {
                let url = format!("http://127.0.0.1:{}/p/", docker::OBICO_PORT);
                let _ = tx.send(("ready".into(), url)).await;
                return;
            }
            docker::ObicoStatus::Unavailable => {
                let _ = tx
                    .send(("fail".into(), "Docker is not available on this host.".into()))
                    .await;
                return;
            }
            _ => {}
        }

        match docker::start_with_progress_channel(tx.clone()).await {
            Ok(()) => {
                let _ = tx
                    .send(("log".into(), "Waiting for ML API to become ready…".into()))
                    .await;
                let ready = docker::wait_for_ready(60).await;
                if ready {
                    let url = format!("http://127.0.0.1:{}/p/", docker::OBICO_PORT);
                    let _ = tx.send(("ready".into(), url)).await;
                } else {
                    let _ = tx
                        .send((
                            "fail".into(),
                            "Container started but ML API did not become ready within 60s."
                                .into(),
                        ))
                        .await;
                }
            }
            Err(e) => {
                let _ = tx.send(("fail".into(), e)).await;
            }
        }
    });

    let stream = futures::stream::unfold(rx, |mut inner_rx| async move {
        inner_rx.recv().await.map(|(ev_type, data)| {
            let event = Event::default().event(ev_type).data(data);
            (Ok::<Event, Infallible>(event), inner_rx)
        })
    });

    Sse::new(stream)
}

pub async fn test_container() -> Json<Value> {
    let ok = docker::wait_for_ready(5).await;
    Json(serde_json::json!({ "ok": ok }))
}

#[derive(Deserialize)]
pub struct TestObicoUrlRequest {
    pub url: String,
}

pub async fn test_url(Json(req): Json<TestObicoUrlRequest>) -> Result<Json<Value>, AppError> {
    let url = req.url.trim();
    if url.is_empty() {
        return Err(AppError::Validation("Obico URL is required".to_string()));
    }

    let client = reqwest::Client::new();
    let res = client
        .get(url)
        .timeout(Duration::from_secs(6))
        .send()
        .await
        .map_err(|e| AppError::Validation(format!("Could not reach Obico URL: {e}")))?;

    Ok(Json(serde_json::json!({
        "ok": true,
        "status": res.status().as_u16(),
    })))
}

pub async fn stop_container() -> Result<Json<Value>, AppError> {
    docker::stop().await.map_err(AppError::Validation)?;
    Ok(Json(serde_json::json!({ "success": true })))
}
