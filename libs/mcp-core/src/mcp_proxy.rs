use crate::models::error::MCPError;
use log::info;
use serde_json::Value;

// This file is kept for backward compatibility
// All process management logic has been moved to infrastructure/process_management/tokio_process_manager.rs
// All domain logic has been moved to domain/services.rs and application/services/tool_service.rs

// Re-export functions from infrastructure/process_management/tokio_process_manager.rs
// for backward compatibility
pub use crate::infrastructure::process_management::TokioProcessManager;
pub use crate::domain::traits::ProcessManager;

// Deprecated: Use ProcessManager::discover_server_tools instead
pub async fn discover_server_tools(
    server_id: &str,
    registry: &mut crate::registry::ToolRegistry,
) -> Result<Vec<Value>, String> {
    info!("discover_server_tools is deprecated, use ProcessManager::discover_server_tools instead");
    
    // This function is deprecated and should not be used
    // Return an error indicating that this function is deprecated
    Err("discover_server_tools is deprecated. Use the ProcessManager trait through AppContext instead.".to_string())
}

// Deprecated: Use ProcessManager::execute_server_tool instead
pub async fn execute_server_tool(
    server_id: &str,
    tool_name: &str,
    parameters: Value,
    registry: &mut crate::registry::ToolRegistry,
) -> Result<Value, MCPError> {
    info!("execute_server_tool is deprecated, use ProcessManager::execute_server_tool instead");
    
    // This function is deprecated and should not be used
    // Return an error indicating that this function is deprecated
    Err(MCPError::UnknownError("execute_server_tool is deprecated. Use the ProcessManager trait through AppContext instead.".to_string()))
}

// Deprecated: Use ProcessManager::spawn_process instead
pub async fn spawn_process(
    configuration: &Value,
    tool_id: &str,
    tool_type: &str,
    env_vars: Option<&std::collections::HashMap<String, String>>,
) -> Result<
    (
        tokio::process::Child,
        tokio::process::ChildStdin,
        tokio::process::ChildStdout,
    ),
    String,
> {
    info!("spawn_process is deprecated, use ProcessManager::spawn_process instead");
    
    // This function is deprecated and should not be used
    // Return an error indicating that this function is deprecated
    Err("spawn_process is deprecated. Use the ProcessManager trait through AppContext instead.".to_string())
}

// Deprecated: Use ProcessManager::kill_process instead
pub async fn kill_process(process: &mut tokio::process::Child) -> Result<(), String> {
    info!("kill_process is deprecated, use ProcessManager::kill_process instead");
    
    // This function is deprecated and should not be used
    // Return an error indicating that this function is deprecated
    Err("kill_process is deprecated. Use the ProcessManager trait through AppContext instead.".to_string())
}
