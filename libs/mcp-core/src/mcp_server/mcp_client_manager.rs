use async_trait::async_trait;
use log::{error, info};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;

use mcp_sdk_core::Tool;

use crate::core::mcp_core::MCPCore;
use crate::core::mcp_core_proxy_ext::McpCoreProxyExt;
use crate::mcp_server::tools::{get_register_server_tool, TOOL_REGISTER_SERVER};
use crate::models::types::{ServerRegistrationRequest, ServerToolInfo};
use crate::registry::registry_service::RegistryService;

use super::handlers::ClientManagerTrait;

/// Client manager implementation that uses MCPCore
pub struct MCPClientManager {
    mcp_core: Arc<MCPCore>,
    // Store the last fetched tools for synchronous access
    tools_cache: RwLock<Vec<Tool>>,
}

impl MCPClientManager {
    /// Create a new MCPClientManager with the given MCPCore instance
    pub fn new(mcp_core: Arc<MCPCore>) -> Self {
        Self {
            mcp_core,
            tools_cache: RwLock::new(Vec::new()),
        }
    }

    /// Convert ServerToolInfo to Tool
    fn convert_to_tool(server_tool: &ServerToolInfo) -> Tool {
        Tool {
            name: server_tool.id.clone(),
            description: server_tool.description.clone(),
            input_schema: serde_json::to_value(server_tool.input_schema.clone())
                .unwrap_or(serde_json::json!({})),
        }
    }

    /// Handle register_server tool call
    async fn handle_register_server(&self, arguments: Value) -> Result<Value, String> {
        // Extract just the tool_id from the arguments
        let tool_id = arguments
            .get("tool_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing or invalid tool_id parameter".to_string())?;

        info!("Fetching tool '{}' from registry", tool_id);

        // Fetch the tool details from the registry
        let registry_tool = RegistryService::get_tool_by_id(tool_id)
            .await
            .map_err(|e| format!("Failed to find tool in registry: {}", e.message))?;

        info!("Found tool '{}' in registry", registry_tool.name);

        // Create the ServerRegistrationRequest from the registry data
        let request = ServerRegistrationRequest {
            server_id: tool_id.to_string(),
            server_name: registry_tool.name.clone(),
            description: registry_tool.description.clone(),
            tools_type: registry_tool.runtime.clone(),
            configuration: Some(registry_tool.config.clone()),
            distribution: Some(registry_tool.distribution.clone()),
        };

        // Call the register_server method from McpCoreProxyExt
        match self.mcp_core.register_server(request).await {
            Ok(response) => {
                info!("Successfully registered server '{}'", tool_id);

                // Update the tools cache to include newly registered tools
                info!("Updating tools cache after new server registration");
                self.update_tools_cache().await;

                // Convert the response to a JSON value
                serde_json::to_value(response)
                    .map_err(|e| format!("Failed to serialize response: {}", e))
            }
            Err(e) => {
                error!("Failed to register server '{}': {}", tool_id, e);
                Err(format!("Failed to register server: {}", e))
            }
        }
    }
}

#[async_trait]
impl ClientManagerTrait for MCPClientManager {
    async fn handle_tool_call(&self, tool_name: String, arguments: Value) -> Result<Value, String> {
        // Check if this is the register_server tool
        if tool_name == TOOL_REGISTER_SERVER {
            info!("Handling register_server tool call");
            return self.handle_register_server(arguments).await;
        }

        // Otherwise, handle as a proxy tool call
        // Parse the tool_name to extract server_id and tool_id
        let parts: Vec<&str> = tool_name.split(':').collect();
        if parts.len() != 2 {
            return Err("Invalid tool_id format. Expected 'server_id:tool_id'".to_string());
        }

        let server_id = parts[0];
        let tool_id = parts[1];

        // Create a tool execution request
        let request = crate::models::types::ToolExecutionRequest {
            tool_id: format!("{}:{}", server_id, tool_id),
            parameters: arguments,
        };

        // Execute the tool
        match self.mcp_core.execute_proxy_tool(request).await {
            Ok(response) => {
                if let Some(result) = response.result {
                    Ok(result)
                } else if let Some(error) = response.error {
                    Err(error)
                } else {
                    Err("No result or error returned from tool execution".to_string())
                }
            }
            Err(e) => Err(format!("Tool execution error: {}", e)),
        }
    }

    async fn list_tools(&self) -> Result<Vec<Tool>, String> {
        // Update the cache first
        self.update_tools_cache().await;

        // Create vector of built-in tools that should always appear first
        let built_in_tools = vec![get_register_server_tool()];

        // Add other built-in tools here as needed
        // Example: built_in_tools.push(get_some_other_built_in_tool());

        // Get the cached tools (user-installed tools)
        let cache = self.tools_cache.read().await;

        // Combine built-in tools with cached tools
        // Built-in tools first, followed by cached tools
        let mut all_tools = built_in_tools;
        all_tools.extend(cache.clone());

        Ok(all_tools)
    }

    fn list_tools_sync(&self) -> Vec<Tool> {
        // Create vector of built-in tools that should always appear first
        let built_in_tools = vec![get_register_server_tool()];

        // Add other built-in tools here as needed
        // Example: built_in_tools.push(get_some_other_built_in_tool());

        // For the synchronous version, we just return whatever installed tools are in the cache
        let installed_tools = match self.tools_cache.try_read() {
            Ok(cache) => cache.clone(),
            Err(_) => {
                // If we can't get the read lock, return empty vec for installed tools
                Vec::new()
            }
        };

        // Combine built-in tools with installed tools
        let mut all_tools = built_in_tools;
        all_tools.extend(installed_tools);

        all_tools
    }

    async fn update_tools_cache(&self) {
        let servers = self.mcp_core.list_servers().await;
        info!("Servers: {:?}", servers);
        info!("Updating tool cache...");
        match self.mcp_core.list_all_server_tools().await {
            Ok(server_tools) => {
                // The server_tools here are only the installed tools from MCPCore
                // Convert ServerToolInfo to Tool for each installed tool
                let installed_tools: Vec<Tool> =
                    server_tools.iter().map(Self::convert_to_tool).collect();

                let mut cache = self.tools_cache.write().await;
                *cache = installed_tools;
                info!("Updated tools cache with {} installed tools", cache.len());
            }
            Err(err) => {
                error!("Error fetching installed tools: {}", err);
            }
        }
    }
}
