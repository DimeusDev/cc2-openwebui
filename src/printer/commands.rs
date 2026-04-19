use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;

/// Shared map of in-flight RPC calls awaiting a printer response.
pub type PendingRpcs = Arc<tokio::sync::Mutex<HashMap<u64, tokio::sync::oneshot::Sender<Value>>>>;

/// Command sent over ws_cmd_tx (9001) or raw_cmd_tx (1883).
#[derive(Debug, Clone)]
pub struct Command {
    pub id: u64,
    pub method: u16,
    pub params: Option<Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;

    #[tokio::test]
    async fn rpc_correlation_delivers_value_and_removes_entry() {
        let pending: PendingRpcs = Arc::new(tokio::sync::Mutex::new(HashMap::new()));
        let (tx, rx) = tokio::sync::oneshot::channel::<Value>();

        pending.lock().await.insert(42, tx);
        assert_eq!(pending.lock().await.len(), 1);

        let expected = json!({ "result": "ok" });
        let to_deliver = expected.clone();

        let pending2 = pending.clone();
        tokio::spawn(async move {
            let sender = pending2.lock().await.remove(&42).unwrap();
            sender.send(to_deliver).ok();
        });

        let received = rx.await.unwrap();
        assert_eq!(received, expected);
        assert_eq!(pending.lock().await.len(), 0);
    }

    #[tokio::test]
    async fn rpc_correlation_dropped_sender_signals_error() {
        let pending: PendingRpcs = Arc::new(tokio::sync::Mutex::new(HashMap::new()));
        let (tx, rx) = tokio::sync::oneshot::channel::<Value>();

        pending.lock().await.insert(1, tx);
        pending.lock().await.remove(&1);

        assert!(rx.await.is_err(), "dropped sender must close the channel");
    }

    #[tokio::test]
    async fn rpc_correlation_unknown_id_is_ignored() {
        let pending: PendingRpcs = Arc::new(tokio::sync::Mutex::new(HashMap::new()));
        let removed = pending.lock().await.remove(&999);
        assert!(removed.is_none());
    }
}
