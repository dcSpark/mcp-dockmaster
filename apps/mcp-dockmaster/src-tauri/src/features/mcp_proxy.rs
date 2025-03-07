use mcp_core::{
    mcp_state::MCPState,
    models::types::{
        DiscoverServerToolsRequest, DiscoverServerToolsResponse, Tool, ToolConfigUpdateRequest,
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
    state: State<'_, MCPState>,
    request: ToolRegistrationRequest,
) -> Result<ToolRegistrationResponse, String> {
    mcp_core::mcp_proxy::register_tool(state.inner(), request).await
}

/// List all registered tools
#[tauri::command]
pub async fn list_tools(state: State<'_, MCPState>) -> Result<Vec<Tool>, String> {
    mcp_core::mcp_proxy::list_tools(state.inner()).await
}

/// List all available tools from all running MCP servers
#[tauri::command]
pub async fn list_all_server_tools(state: State<'_, MCPState>) -> Result<Vec<Tool>, String> {
    mcp_core::mcp_proxy::list_all_server_tools(state.inner()).await
}

/// Discover tools from a specific MCP server
#[tauri::command]
pub async fn discover_tools(
    state: State<'_, MCPState>,
    request: DiscoverServerToolsRequest,
) -> Result<DiscoverServerToolsResponse, String> {
    mcp_core::mcp_proxy::discover_tools(state.inner(), request).await
}

/// Execute a tool from an MCP server
#[tauri::command]
pub async fn execute_proxy_tool(
    state: State<'_, MCPState>,
    request: ToolExecutionRequest,
) -> Result<ToolExecutionResponse, String> {
    mcp_core::mcp_proxy::execute_proxy_tool(state.inner(), request).await
}

/// Update a tool's status (enabled/disabled)
#[tauri::command]
pub async fn update_tool_status(
    state: State<'_, MCPState>,
    request: ToolUpdateRequest,
) -> Result<ToolUpdateResponse, String> {
    mcp_core::mcp_proxy::update_tool_status(state.inner(), request).await
}

/// Update a tool's configuration (environment variables)
#[tauri::command]
pub async fn update_tool_config(
    state: State<'_, MCPState>,
    request: ToolConfigUpdateRequest,
) -> Result<ToolConfigUpdateResponse, String> {
    mcp_core::mcp_proxy::update_tool_config(state.inner(), request).await
}

/// Uninstall a registered tool
#[tauri::command]
pub async fn uninstall_tool(
    state: State<'_, MCPState>,
    request: ToolUninstallRequest,
) -> Result<ToolUninstallResponse, String> {
    mcp_core::mcp_proxy::uninstall_tool(state.inner(), request).await
}

/// Get all server data in a single function to avoid multiple locks
#[tauri::command]
pub async fn get_all_server_data(state: State<'_, MCPState>) -> Result<Value, String> {
    mcp_core::mcp_proxy::get_all_server_data(state.inner()).await
}

/// Save the MCP state to the database
#[tauri::command]
pub async fn save_mcp_state_command(_state: State<'_, MCPState>) -> Result<String, String> {
    println!("Saving MCP state to the database is no longer supported");
    Ok("Saving MCP state to the database is no longer supported".to_string())
    // mcp_core::mcp_proxy::save_mcp_state_command(state.inner()).await
}

/// Load MCP state from the database
#[tauri::command]
pub async fn load_mcp_state_command(_state: State<'_, MCPState>) -> Result<String, String> {
    println!("Loading MCP state from the database is no longer supported");
    Ok("Loading MCP state from the database is no longer supported".to_string())
    // mcp_core::mcp_proxy::load_mcp_state_command(state.inner()).await
}

/// Check if the database exists and has data
#[tauri::command]
pub async fn check_database_exists_command() -> Result<bool, String> {
    mcp_core::mcp_proxy::check_database_exists_command().await
}

/// Clear all data from the database
#[tauri::command]
pub async fn clear_database_command() -> Result<String, String> {
    mcp_core::mcp_proxy::clear_database_command().await
}

/// Restart a tool by its ID
#[tauri::command(rename_all = "camelCase")]
pub async fn restart_tool_command(
    state: State<'_, MCPState>,
    tool_id: String,
) -> Result<ToolUpdateResponse, String> {
    mcp_core::mcp_proxy::restart_tool_command(state.inner(), tool_id).await
}
