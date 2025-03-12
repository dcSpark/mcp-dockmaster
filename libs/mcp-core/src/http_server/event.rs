use crate::models::event::EventEmitter;
use log::info;
use serde_json::Value;

/// HTTP server implementation of EventEmitter
/// 
/// This implementation logs events but doesn't actually emit them
/// since the HTTP server doesn't have a UI to update.
#[derive(Clone, Default)]
pub struct HttpEventEmitter;

impl EventEmitter for HttpEventEmitter {
    fn emit(&self, event: &str, payload: Value) -> Result<(), String> {
        info!("HTTP Server Event '{}' with payload: {}", event, payload);
        Ok(())
    }
}
