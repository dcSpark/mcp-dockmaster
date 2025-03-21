use serde_json::json;
use super::session_manager::SESSION_MANAGER;

pub async fn broadcast_tools_list_changed() {
    let notification = json!({
        "jsonrpc": "2.0",
        "method": "notifications/tools/list_changed",
        "params": {}
    });

    let message = serde_json::to_string(&notification).unwrap_or_default();
    let failed = SESSION_MANAGER.broadcast_message(&message).await;
    
    if !failed.is_empty() {
        log::warn!("Failed to send tools list changed notification to some sessions: {:?}", failed);
    }
}
