use crate::models::types::ServerToolInfo;
use crate::registry::server_registry::ServerRegistry;
use crate::MCPError;
use log::{error, info};
use mcp_sdk_client::transport::stdio::StdioTransport;
use mcp_sdk_client::transport::stdio::StdioTransportHandle;
use mcp_sdk_client::{
    ClientCapabilities, ClientInfo, McpClient, McpClientTrait, McpService, Transport,
};
use mcp_sdk_core::Tool;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// Type alias for a transport that uses StdioTransportHandle
pub type StdioTransportType = Arc<dyn Transport<Handle = StdioTransportHandle> + Send + Sync>;

/// Type alias for the transport manager
pub type TransportManagerType = Arc<RwLock<HashMap<String, StdioTransportType>>>;

/// Type alias for McpClient trait objects
pub type McpClientType = Arc<dyn McpClientTrait + Send + Sync>;

/// MCPState: the main service layer
///
/// This module coordinates database operations, process management, and discovered tools.
/// It serves as the central orchestration layer that connects the database (ServerRegistry),
/// process management (ProcessManager), and JSON-RPC operations (mcp_proxy).
#[derive(Clone)]
pub struct MCPState {
    pub tool_registry: Arc<RwLock<ServerRegistry>>,
    pub server_tools: Arc<RwLock<HashMap<String, Vec<ServerToolInfo>>>>,
    pub clients: Arc<RwLock<HashMap<String, McpClientType>>>,
    pub transport_manager: TransportManagerType,
}

impl MCPState {
    pub fn new(
        tool_registry: Arc<RwLock<ServerRegistry>>,
        server_tools: Arc<RwLock<HashMap<String, Vec<ServerToolInfo>>>>,
        clients: Arc<RwLock<HashMap<String, McpClientType>>>,
        transport_manager: TransportManagerType,
    ) -> Self {
        Self {
            tool_registry,
            server_tools,
            clients,
            transport_manager,
        }
    }

    /// Kill all running processes
    pub async fn kill_all_processes(&self) {
        for transport in self.transport_manager.read().await.values() {
            let _ = transport.close().await;
        }
    }

    /// Kill a process by its ID
    pub async fn kill_process(&self, server_id: &str) {
        let transport = self.transport_manager.read().await.get(server_id).cloned();
        if let Some(transport) = transport {
            let _ = transport.close().await;
            self.transport_manager.write().await.remove(server_id);
            self.clients.write().await.remove(server_id);
            self.server_tools.write().await.remove(server_id);
            let _ = self.tool_registry.write().await.delete_server(server_id);
        }
    }

    /// Execute a tool on a server
    pub async fn execute_tool(
        &self,
        server_id: &str,
        tool_id: &str,
        parameters: Value,
    ) -> Result<Value, MCPError> {
        let client = self.clients.read().await.get(server_id).cloned();
        if let Some(client) = client {
            info!("Successfully got client for server: {}", server_id);
            let result = client
                .call_tool(tool_id, parameters)
                .await
                .map_err(|e| MCPError::ToolExecutionError(e.to_string()))?;

            // Convert the result to a Value
            let content_value = serde_json::to_value(result.content)
                .map_err(|e| MCPError::SerializationError(e.to_string()))?;

            Ok(content_value)
        } else {
            Err(MCPError::ServerNotFound(format!(
                "No client found for server: {}",
                server_id
            )))
        }
    }

    /// Restart a server by its ID
    pub async fn restart_server(&self, server_id: &str) -> Result<(), String> {
        info!("Attempting to restart server: {}", server_id);

        // Get tool from database
        let server_data = {
            let registry = self.tool_registry.read().await;
            registry.get_server(server_id)?
        };

        // Check if tools_type is empty
        if server_data.tools_type.is_empty() {
            error!("Missing tools_type for server {}", server_id);
            return Err(format!("Missing tools_type for server {}", server_id));
        }

        let client = self.clients.read().await.get(server_id).cloned();
        if let Some(_client) = client {
            info!("Successfully got client for server: {}", server_id);
            Ok(())
        } else {
            // Check if the tool is enabled
            if !server_data.enabled {
                info!("Server {} is disabled, not restarting", server_id);
                return Ok(());
            }

            // Extract environment variables from the tool configuration
            let env_vars = if let Some(configuration) = &server_data.configuration {
                if let Some(env_map) = &configuration.env {
                    info!(
                        "Extracted {} environment variables for server {}",
                        env_map.len(),
                        server_id
                    );
                    // Convert ToolEnvironment -> just the defaults
                    let simple_env_map: HashMap<String, String> = env_map
                        .iter()
                        .filter_map(|(k, tool_env)| {
                            tool_env.default.clone().map(|v| (k.clone(), v))
                        })
                        .collect();
                    Some(simple_env_map)
                } else {
                    info!("No environment variables found for server {}", server_id);
                    None
                }
            } else {
                info!("No configuration found for server {}", server_id);
                None
            };

            // Get the configuration from the tool data
            let config_value = if let Some(configuration) = &server_data.configuration {
                info!("Using configuration from server data for {}", server_id);
                json!({
                    "command": configuration.command,
                    "args": configuration.args
                })
            } else if !server_data
                .entry_point
                .clone()
                .unwrap_or_default()
                .is_empty()
            {
                info!(
                    "Creating simple configuration with entry_point for {}",
                    server_id
                );
                json!({
                    "command": server_data.entry_point
                })
            } else {
                error!("Missing configuration for server {}", server_id);
                return Err(format!("Missing configuration for server {}", server_id));
            };

            let transport = StdioTransport::new(
                config_value["command"].as_str().unwrap().to_string(),
                config_value["args"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_str().unwrap().to_string())
                    .collect(),
                env_vars.unwrap_or_default(),
            );

            let transport_handle = match transport.start().await {
                Ok(handle) => handle,
                Err(e) => return Err(format!("Failed to start transport: {}", e)),
            };

            // Create the service with a timeout of 300 seconds
            let service = McpService::with_timeout(transport_handle, Duration::from_secs(300));

            // Create and initialize the client
            let mut client = McpClient::new(service);

            let client_info = ClientInfo {
                name: "mcp-dockmaster".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            };

            if let Err(e) = client
                .initialize(client_info, ClientCapabilities::default())
                .await
            {
                return Err(format!("Failed to initialize client: {}", e));
            }

            self.clients
                .write()
                .await
                .insert(server_id.to_string(), Arc::new(client));

            self.transport_manager
                .write()
                .await
                .insert(server_id.to_string(), Arc::new(transport));

            let _ = self.discover_server_tools(server_id).await;

            info!("Successfully initialized client for server: {}", server_id);

            Ok(())
        }
    }

    /// Discover tools from a server
    pub async fn discover_server_tools(&self, server_id: &str) -> Result<Vec<Tool>, String> {
        let client = self.clients.read().await.get(server_id).cloned();
        if let Some(client) = client {
            info!("Successfully got client for server: {}", server_id);
            let list_tools = client.list_tools(None).await.map_err(|e| e.to_string())?;
            let tools = list_tools.tools;
            info!(
                "Successfully discovered {} tools for {}",
                tools.len(),
                server_id
            );

            // Save the tools to the database
            let registry = self.tool_registry.read().await;
            for tool in &tools {
                let tool_info = ServerToolInfo {
                    id: tool.name.clone(),
                    name: tool.name.clone(),
                    description: tool.description.clone(),
                    input_schema: if tool.input_schema.is_object() {
                        serde_json::from_value(tool.input_schema.clone()).ok()
                    } else {
                        None
                    },
                    proxy_id: Some(server_id.to_string()),
                    server_id: server_id.to_string(),
                    is_active: true,
                };
                info!("Saving tool info to database: {:?}", tool_info);
                if let Err(e) = registry.save_server_tool(&tool_info) {
                    error!("Failed to save server tool to database: {}", e);
                }
            }

            Ok(tools)
        } else {
            info!("No client found for server: {}", server_id);
            Err(format!("No client found for server: {}", server_id))
        }
    }
}
