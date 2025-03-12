use mcp_core::models::event::EventEmitter;
use serde_json::Value;
use tauri::AppHandle;
use tauri::Emitter;

/// TauriEventEmitter: Implementation of EventEmitter for Tauri
///
/// This struct wraps a Tauri AppHandle and implements the EventEmitter trait
/// to emit events to the frontend using Tauri's event system.
#[derive(Clone)]
pub struct TauriEventEmitter {
    app_handle: AppHandle,
}

impl TauriEventEmitter {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }
}

impl EventEmitter for TauriEventEmitter {
    fn emit(&self, event: &str, payload: Value) -> Result<(), String> {
        self.app_handle
            .emit(event, payload)
            .map_err(|e| format!("Failed to emit event: {}", e))
    }
}
