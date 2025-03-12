use crate::models::types::ServerStatus;
use serde_json::{json, Value};
use std::fmt;

/// EventEmitter trait for abstracting event emission
/// 
/// This trait allows the core library to emit events without
/// directly depending on any specific event system implementation.
pub trait EventEmitter {
    /// Emit an event with a payload
    fn emit(&self, event: &str, payload: Value) -> Result<(), String>;
}

/// Default implementation that logs events but doesn't actually emit them
#[derive(Clone, Default)]
pub struct LoggingEventEmitter;

impl EventEmitter for LoggingEventEmitter {
    fn emit(&self, event: &str, payload: Value) -> Result<(), String> {
        log::info!("Event '{}' would be emitted with payload: {}", event, payload);
        Ok(())
    }
}

/// Server status change event
pub struct ServerStatusChangeEvent<'a> {
    pub server_id: &'a str,
    pub status: &'a ServerStatus,
}

impl<'a> ServerStatusChangeEvent<'a> {
    pub fn new(server_id: &'a str, status: &'a ServerStatus) -> Self {
        Self { server_id, status }
    }

    pub fn to_payload(&self) -> Value {
        json!({
            "server_id": self.server_id,
            "status": self.status
        })
    }
}

impl<'a> fmt::Display for ServerStatusChangeEvent<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ServerStatusChange(server_id: {}, status: {})", 
            self.server_id, self.status)
    }
}

/// Helper function to emit server status change events
pub fn emit_server_status_change<E: EventEmitter>(
    emitter: &E, 
    server_id: &str, 
    status: &ServerStatus
) -> Result<(), String> {
    let event = ServerStatusChangeEvent::new(server_id, status);
    log::info!("Emitting event: {}", event);
    emitter.emit("server-status-changed", event.to_payload())
}
