use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS};
use std::time::Duration;
use tokio::time::timeout;
use tracing::{debug, info, trace};

use crate::error::PrinterError;

pub async fn discover_printer_id(
    ip: &str,
    username: &str,
    password: &str,
    timeout_secs: u64,
) -> Result<String, PrinterError> {
    use rand::Rng;
    let suffix: String = (0..4).map(|_| rand::thread_rng().gen_range(0..10u8).to_string()).collect();
    let client_id = format!("cc2_disc_{suffix}");

    let mut mqtt_options = MqttOptions::new(&client_id, ip, 1883);
    mqtt_options.set_credentials(username, password);
    mqtt_options.set_keep_alive(Duration::from_secs(60));
    mqtt_options.set_clean_session(true);

    let (client, mut eventloop) = AsyncClient::new(mqtt_options, 10);

    info!("connecting to {ip}:1883 for printer ID discovery");

    let discovery_topics = [
        "elegoo/+/+/api_request",
        "elegoo/+/+/api_response",
        "elegoo/+/api_status",
    ];

    for topic in discovery_topics {
        client
            .subscribe(topic, QoS::AtMostOnce)
            .await
            .map_err(|e| PrinterError::Registration(format!("subscribe failed for {topic}: {e}")))?;

        debug!("subscribed to discovery topic pattern: {topic}");
    }

    let result = timeout(
        Duration::from_secs(timeout_secs),
        wait_for_discovery_message(&mut eventloop),
    )
    .await;

    client.disconnect().await.ok();

    match result {
        Ok(Ok(printer_id)) => {
            info!("discovered printer ID: {printer_id}");
            Ok(printer_id)
        }
        Ok(Err(e)) => Err(e),
        Err(_) => Err(PrinterError::DiscoveryTimeout(timeout_secs)),
    }
}

async fn wait_for_discovery_message(eventloop: &mut rumqttc::EventLoop,) -> Result<String, PrinterError> {
    loop {
        let notification = eventloop
            .poll()
            .await
            .map_err(|e| PrinterError::Registration(format!("eventloop error: {e}")))?;

        if let Event::Incoming(Incoming::Publish(publish)) = notification {
            trace!("discovery received message on topic: {}", publish.topic);

            if let Some(printer_id) = extract_printer_id_from_topic(&publish.topic) {
                info!(
                    "discovered printer ID from topic '{}': {}",
                    publish.topic, printer_id
                );

                return Ok(printer_id);
            }
        }
    }
}

fn extract_printer_id_from_topic(topic: &str) -> Option<String> {
    let parts: Vec<&str> = topic.split('/').collect();

    match parts.as_slice() {
        // elegoo/XXX/api_status
        ["elegoo", printer_id, "api_status"] if !printer_id.is_empty() => {
            Some((*printer_id).to_string())
        }

        // elegoo/XXX/xxx/api_request
        ["elegoo", printer_id, _subtopic, "api_request"] if !printer_id.is_empty() => {
            Some((*printer_id).to_string())
        }

        // elegoo/XXX/xxx/api_response
        ["elegoo", printer_id, _subtopic, "api_response"] if !printer_id.is_empty() => {
            Some((*printer_id).to_string())
        }

        _ => None,
    }
}
