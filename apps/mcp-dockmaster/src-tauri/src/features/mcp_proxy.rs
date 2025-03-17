use mcp_core::{
    core::{
        mcp_core::MCPCore, mcp_core_database_ext::McpCoreDatabaseExt,
        mcp_core_installers_ext::McpCoreInstallersExt, mcp_core_proxy_ext::McpCoreProxyExt,
    },
    models::types::{
        DiscoverServerToolsRequest, ServerConfigUpdateRequest, ServerRegistrationRequest,
        ServerRegistrationResponse, ServerToolInfo, ServerUninstallResponse, ServerUpdateRequest,
        ToolConfigUpdateResponse, ToolExecutionRequest, ToolExecutionResponse,
        ToolUninstallRequest, ToolUpdateResponse,
    },
    types::{IsProcessRunningRequest, RuntimeServer},
};
use tauri::State;

/// Register a new tool with the MCP server
#[tauri::command]
pub async fn register_server(
    mcp_core: State<'_, MCPCore>,
    request: ServerRegistrationRequest,
) -> Result<ServerRegistrationResponse, String> {
    mcp_core.register_server(request).await
}

/// List all registered tools
#[tauri::command]
pub async fn list_servers(mcp_core: State<'_, MCPCore>) -> Result<Vec<RuntimeServer>, String> {
    mcp_core.list_servers().await
}

/// List all available tools from all running MCP servers
/// If tools are hidden in the state, returns an empty list of tools
#[tauri::command]
pub async fn list_all_server_tools(
    mcp_core: State<'_, MCPCore>,
) -> Result<Vec<ServerToolInfo>, String> {
    mcp_core.list_all_server_tools().await
}

/// Discover tools from a specific MCP server
#[tauri::command]
pub async fn discover_tools(
    mcp_core: State<'_, MCPCore>,
    request: DiscoverServerToolsRequest,
) -> Result<Vec<ServerToolInfo>, String> {
    mcp_core.list_server_tools(request).await
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
pub async fn update_server_status(
    mcp_core: State<'_, MCPCore>,
    request: ServerUpdateRequest,
) -> Result<ToolUpdateResponse, String> {
    mcp_core.update_server_status(request).await
}

/// Update a tool's configuration (environment variables)
#[tauri::command]
pub async fn update_server_config(
    mcp_core: State<'_, MCPCore>,
    request: ServerConfigUpdateRequest,
) -> Result<ToolConfigUpdateResponse, String> {
    mcp_core.update_server_config(request).await
}

/// Uninstall a registered tool
#[tauri::command]
pub async fn uninstall_server(
    mcp_core: State<'_, MCPCore>,
    request: ToolUninstallRequest,
) -> Result<ServerUninstallResponse, String> {
    mcp_core.uninstall_server(request).await
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
pub async fn restart_server_command(
    mcp_core: State<'_, MCPCore>,
    server_id: String,
) -> Result<ToolUpdateResponse, String> {
    mcp_core.restart_server_command(server_id).await
}

// Check if Claude is installed
#[tauri::command]
pub async fn check_claude_installed(mcp_core: State<'_, MCPCore>) -> Result<bool, String> {
    mcp_core.is_claude_installed()
}

// Check if Cursor is installed
#[tauri::command]
pub async fn check_cursor_installed(mcp_core: State<'_, MCPCore>) -> Result<bool, String> {
    mcp_core.is_cursor_installed()
}

// Install Claude
#[tauri::command]
pub async fn install_claude(mcp_core: State<'_, MCPCore>) -> Result<(), String> {
    mcp_core.install_claude()
}

// Install Cursor
#[tauri::command]
pub async fn install_cursor(mcp_core: State<'_, MCPCore>) -> Result<(), String> {
    mcp_core.install_cursor()
}

// Get Claude config
#[tauri::command]
pub async fn get_claude_config(mcp_core: State<'_, MCPCore>) -> Result<String, String> {
    mcp_core.get_claude_config()
}

// Get Cursor config
#[tauri::command]
pub async fn get_cursor_config(mcp_core: State<'_, MCPCore>) -> Result<String, String> {
    mcp_core.get_cursor_config()
}

// Get Generic config
#[tauri::command]
pub async fn get_generic_config(mcp_core: State<'_, MCPCore>) -> Result<String, String> {
    mcp_core.get_generic_config()
}

/// Import a server from a GitHub repository URL
#[tauri::command]
pub async fn import_server_from_url(
    mcp_core: State<'_, MCPCore>,
    url: String,
) -> Result<ServerRegistrationResponse, String> {
    mcp_core.import_server_from_url(url).await
}

/// Set the tool visibility state
#[tauri::command]
pub async fn set_tools_hidden(
    mcp_core: State<'_, MCPCore>,
    hidden: bool,
) -> Result<(), String> {
    let mcp_state = mcp_core.mcp_state.read().await;
    mcp_state.set_tools_hidden(hidden).await
}

#[tauri::command]
pub async fn restart_process(
    mcp_core: State<'_, MCPCore>,
    process: IsProcessRunningRequest,
) -> Result<bool, String> {
    mcp_core.restart_process(process.process_name.as_str())
}

#[tauri::command]
pub async fn is_process_running(
    mcp_core: State<'_, MCPCore>,
    process: IsProcessRunningRequest,
) -> Result<bool, String> {
    mcp_core.is_process_running(process.process_name.as_str())
}
