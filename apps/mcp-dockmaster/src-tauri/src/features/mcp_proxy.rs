use mcp_core::{
    core::{
        mcp_core::MCPCore, mcp_core_database_ext::McpCoreDatabaseExt,
        mcp_core_proxy_ext::McpCoreProxyExt,
    },
    models::types::{
        DiscoverServerToolsRequest, DiscoverServerToolsResponse, ToolConfigUpdateRequest,
        ToolConfigUpdateResponse, ToolExecutionRequest, ToolExecutionResponse,
        ToolRegistrationRequest, ToolRegistrationResponse, ToolUninstallRequest,
        ToolUninstallResponse, ToolUpdateRequest, ToolUpdateResponse,
    },
};
use serde_json::Value;
use tauri::State;

/// Register a new tool with the MCP server
#[tauri::command]
pub async fn register_tool(
    mcp_core: State<'_, MCPCore>,
    request: ToolRegistrationRequest,
) -> Result<ToolRegistrationResponse, String> {
    mcp_core.register_tool(request).await
}

/// List all registered tools
#[tauri::command]
pub async fn list_tools(mcp_core: State<'_, MCPCore>) -> Result<Vec<Value>, String> {
    mcp_core.list_tools().await
}

/// List all available tools from all running MCP servers
#[tauri::command]
pub async fn list_all_server_tools(mcp_core: State<'_, MCPCore>) -> Result<Vec<Value>, String> {
    mcp_core.list_all_server_tools().await
}

/// Discover tools from a specific MCP server
#[tauri::command]
pub async fn discover_tools(
    mcp_core: State<'_, MCPCore>,
    request: DiscoverServerToolsRequest,
) -> Result<DiscoverServerToolsResponse, String> {
    mcp_core.discover_tools(request).await
}

/// Execute a tool from an MCP server
#[tauri::command]
pub async fn execute_proxy_tool(
    mcp_core: State<'_, MCPCore>,
    request: ToolExecutionRequest,
) -> Result<ToolExecutionResponse, String> {
    mcp_core.execute_proxy_tool(request).await
}

/// Update a tool's status (enabled/disabled)
#[tauri::command]
pub async fn update_tool_status(
    mcp_core: State<'_, MCPCore>,
    request: ToolUpdateRequest,
) -> Result<ToolUpdateResponse, String> {
    mcp_core.update_tool_status(request).await
}

/// Update a tool's configuration (environment variables)
#[tauri::command]
pub async fn update_tool_config(
    mcp_core: State<'_, MCPCore>,
    request: ToolConfigUpdateRequest,
) -> Result<ToolConfigUpdateResponse, String> {
    mcp_core.update_tool_config(request).await
}

/// Uninstall a registered tool
#[tauri::command]
pub async fn uninstall_tool(
    mcp_core: State<'_, MCPCore>,
    request: ToolUninstallRequest,
) -> Result<ToolUninstallResponse, String> {
    mcp_core.uninstall_tool(request).await
}

/// Get all server data in a single function to avoid multiple locks
#[tauri::command]
pub async fn get_all_server_data(mcp_core: State<'_, MCPCore>) -> Result<Value, String> {
    mcp_core.get_all_server_data().await
}

/// Save the MCP state to the database
#[tauri::command]
pub async fn save_mcp_state_command(_state: State<'_, MCPCore>) -> Result<String, String> {
    println!("Saving MCP state to the database is no longer supported");
    Ok("Saving MCP state to the database is no longer supported".to_string())
    // mcp_core::mcp_proxy::save_mcp_state_command(state.inner()).await
}

/// Load MCP state from the database
#[tauri::command]
pub async fn load_mcp_state_command(_state: State<'_, MCPCore>) -> Result<String, String> {
    println!("Loading MCP state from the database is no longer supported");
    Ok("Loading MCP state from the database is no longer supported".to_string())
    // mcp_core::mcp_proxy::load_mcp_state_command(state.inner()).await
}

/// Check if the database exists and has data
#[tauri::command]
pub async fn check_database_exists_command(mcp_core: State<'_, MCPCore>) -> Result<bool, String> {
    mcp_core.check_database_exists().await
}

/// Clear all data from the database
#[tauri::command]
pub async fn clear_database_command(mcp_core: State<'_, MCPCore>) -> Result<(), String> {
    mcp_core.clear_database().await
}

/// Restart a tool by its ID
#[tauri::command(rename_all = "camelCase")]
pub async fn restart_tool_command(
    mcp_core: State<'_, MCPCore>,
    tool_id: String,
) -> Result<ToolUpdateResponse, String> {
    mcp_core.restart_tool_command(tool_id).await
}
