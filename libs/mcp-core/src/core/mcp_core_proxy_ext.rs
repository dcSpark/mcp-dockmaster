use crate::mcp_protocol::{discover_server_tools, execute_server_tool};
use crate::mcp_state::mcp_state::McpStateProcessMonitor;
use crate::mcp_state::mcp_state_process_utils::{kill_process, spawn_process};
use crate::models::types::{
    DiscoverServerToolsRequest, DiscoverServerToolsResponse, Distribution, Tool,
    ToolConfigUpdateRequest, ToolConfigUpdateResponse, ToolConfiguration, ToolEnvironment,
    ToolExecutionRequest, ToolExecutionResponse, ToolRegistrationRequest, ToolRegistrationResponse,
    ToolUninstallRequest, ToolUninstallResponse, ToolUpdateRequest, ToolUpdateResponse,
};
use crate::MCPError;
use anyhow::Result;
use futures::future;
use log::{error, info};
use serde_json::{json, Value};
use std::collections::HashMap;
use tokio::time::Duration;

use super::mcp_core::MCPCore;

pub trait McpCoreProxyExt {
    fn register_tool(
        &self,
        tool: ToolRegistrationRequest,
    ) -> impl std::future::Future<Output = Result<ToolRegistrationResponse, String>> + Send;
    fn list_tools(&self) -> impl std::future::Future<Output = Result<Vec<Value>, String>> + Send;
    fn list_all_server_tools(
        &self,
    ) -> impl std::future::Future<Output = Result<Vec<Value>, String>> + Send;
    fn discover_tools(
        &self,
        request: DiscoverServerToolsRequest,
    ) -> impl std::future::Future<Output = Result<DiscoverServerToolsResponse, String>> + Send;
    fn execute_proxy_tool(
        &self,
        request: ToolExecutionRequest,
    ) -> impl std::future::Future<Output = Result<ToolExecutionResponse, String>> + Send;
    fn update_tool_status(
        &self,
        request: ToolUpdateRequest,
    ) -> impl std::future::Future<Output = Result<ToolUpdateResponse, String>> + Send;
    fn update_tool_config(
        &self,
        request: ToolConfigUpdateRequest,
    ) -> impl std::future::Future<Output = Result<ToolConfigUpdateResponse, String>> + Send;
    fn uninstall_tool(
        &self,
        request: ToolUninstallRequest,
    ) -> impl std::future::Future<Output = Result<ToolUninstallResponse, String>> + Send;
    fn get_all_server_data(
        &self,
    ) -> impl std::future::Future<Output = Result<Value, String>> + Send;
    fn restart_tool_command(
        &self,
        tool_id: String,
    ) -> impl std::future::Future<Output = Result<ToolUpdateResponse, String>> + Send;
    fn init_mcp_server(&self) -> impl std::future::Future<Output = Result<()>> + Send;
    fn kill_all_processes(&self) -> impl std::future::Future<Output = Result<()>> + Send;
}

impl McpCoreProxyExt for MCPCore {
    /// Register a new tool with the MCP server
    async fn register_tool(
        &self,
        request: ToolRegistrationRequest,
    ) -> Result<ToolRegistrationResponse, String> {
        info!("Starting registration of tool: {}", request.tool_name);

        // Safely access the command field if configuration exists
        if let Some(config) = &request.configuration {
            if let Some(cmd) = config.get("command") {
                info!("Command: {}", cmd);
            } else {
                info!("Command: Not specified in configuration");
            }
        } else {
            info!("Configuration not provided");
        }

        let registry = self.tool_registry.write().await;

        // Generate a simple tool ID (in production, use UUIDs)
        let tool_id = format!("tool_{}", registry.get_all_tools()?.len() + 1);
        info!("Generated tool ID: {}", tool_id);

        // Create the tool configuration if provided
        let configuration = request
            .configuration
            .as_ref()
            .map(|config| ToolConfiguration {
                command: config
                    .get("command")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                args: config.get("args").and_then(|v| v.as_array()).map(|args| {
                    args.iter()
                        .filter_map(|arg| arg.as_str().map(|s| s.to_string()))
                        .collect()
                }),
                env: config.get("env").and_then(|v| v.as_object()).map(|env| {
                    env.iter()
                        .map(|(k, v)| {
                            let description = if let Value::Object(obj) = v {
                                obj.get("description")
                                    .and_then(|d| d.as_str())
                                    .unwrap_or_default()
                                    .to_string()
                            } else {
                                "".to_string()
                            };

                            let default = if let Value::Object(obj) = v {
                                obj.get("default").and_then(|d| match d {
                                    Value::String(s) => Some(s.clone()),
                                    Value::Number(n) => Some(n.to_string()),
                                    Value::Bool(b) => Some(b.to_string()),
                                    _ => None,
                                })
                            } else if let Value::String(s) = v {
                                Some(s.clone())
                            } else if let Value::Number(n) = v {
                                Some(n.to_string())
                            } else if let Value::Bool(b) = v {
                                Some(b.to_string())
                            } else {
                                None
                            };

                            let required = if let Value::Object(obj) = v {
                                obj.get("required")
                                    .and_then(|r| r.as_bool())
                                    .unwrap_or(false)
                            } else {
                                false
                            };

                            (
                                k.to_string(),
                                ToolEnvironment {
                                    description,
                                    default,
                                    required,
                                },
                            )
                        })
                        .collect()
                }),
            });

        // Environment variables are now handled in the configuration field

        // Create the Tool struct
        let tool = Tool {
            name: request.tool_name.clone(),
            description: request.description.clone(),
            enabled: true, // Default to enabled
            tool_type: request.tool_type.clone(),
            entry_point: None,
            configuration,
            distribution: request.distribution.as_ref().map(|v| {
                serde_json::from_value(v.clone()).unwrap_or(Distribution {
                    r#type: "".to_string(),
                    package: "".to_string(),
                })
            }),
        };

        // Save the tool in the registry
        registry.save_tool(&tool_id, &tool)?;

        let mcp_state_clone = self.mcp_state.clone();
        {
            // Create a default empty tools list
            let mcp_state = mcp_state_clone.write().await;
            let mut server_tools = mcp_state.server_tools.write().await;
            server_tools.insert(tool_id.clone(), Vec::new());
        }

        // Extract environment variables from the tool configuration
        let env_vars = if let Some(configuration) = &tool.configuration {
            configuration.env.as_ref().map(|map| {
                // Convert ToolEnvironment -> just the defaults
                map.iter()
                    .filter_map(|(k, tool_env)| tool_env.default.clone().map(|v| (k.clone(), v)))
                    .collect::<HashMap<String, String>>()
            })
        } else {
            None
        };

        // Create the config_value for the spawn functions
        let config_value = if let Some(configuration) = &tool.configuration {
            json!({
                "command": configuration.command,
                "args": configuration.args
            })
        } else {
            return Err("Configuration is required for tools".to_string());
        };

        // Spawn process based on tool type
        let spawn_result = spawn_process(
            &config_value,
            &tool_id,
            &request.tool_type,
            env_vars.as_ref(),
        )
        .await;

        let mcp_state_clone = self.mcp_state.clone();
        match spawn_result {
            Ok((process, stdin, stdout)) => {
                info!("Process spawned successfully for tool ID: {}", tool_id);
                {
                    let mcp_state = mcp_state_clone.write().await;
                    let mut process_manager = mcp_state.process_manager.write().await;
                    process_manager
                        .processes
                        .insert(tool_id.clone(), Some(process));
                    process_manager
                        .process_ios
                        .insert(tool_id.clone(), (stdin, stdout));
                }
                // Wait a moment for the server to start
                info!("Waiting for server to initialize...");
                tokio::time::sleep(Duration::from_secs(3)).await;

                // Try to discover tools from this server with a timeout to avoid hanging
                info!("Attempting to discover tools from server {}", tool_id);
                let discover_result = tokio::time::timeout(Duration::from_secs(15), async {
                    let mcp_state = mcp_state_clone.write().await;
                    let mut process_manager = mcp_state.process_manager.write().await;
                    if let Some((stdin, stdout)) = process_manager.process_ios.get_mut(&tool_id) {
                        discover_server_tools(&tool_id, stdin, stdout).await
                    } else {
                        Err(format!("Server {} not found or not running", tool_id))
                    }
                })
                .await;

                // Handle the result of the discovery attempt
                match discover_result {
                    Ok(Ok(tools)) => {
                        info!(
                            "Successfully discovered {} tools from {}",
                            tools.len(),
                            tool_id
                        );
                        // Clone tools before inserting to avoid the "moved value" error
                        let tools_clone = tools.clone();
                        {
                            let mcp_state = mcp_state_clone.write().await;
                            let mut server_tools = mcp_state.server_tools.write().await;
                            server_tools.insert(tool_id.clone(), tools);

                            // If empty tools, add a default "main" tool
                            if tools_clone.is_empty() {
                                info!("No tools discovered, adding a default main tool");
                                let default_tool = json!({
                                    "id": "main",
                                    "name": request.tool_name,
                                    "description": request.description
                                });
                                server_tools.insert(tool_id.clone(), vec![default_tool]);
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        error!("Error discovering tools from server {}: {}", tool_id, e);
                        {
                            // Add a default tool since discovery failed
                            let mcp_state = mcp_state_clone.write().await;
                            let mut server_tools = mcp_state.server_tools.write().await;
                            let default_tool = json!({
                                "id": "main",
                                "name": request.tool_name,
                                "description": request.description
                            });
                            server_tools.insert(tool_id.clone(), vec![default_tool]);
                            info!("Added default tool for server {}", tool_id);
                        }
                    }
                    Err(_) => {
                        error!("Timeout while discovering tools from server {}", tool_id);
                        // Add a default tool since discovery timed out
                        let mcp_state = mcp_state_clone.write().await;
                        let mut server_tools = mcp_state.server_tools.write().await;
                        let default_tool = json!({
                            "id": "main",
                            "name": request.tool_name,
                            "description": request.description
                        });
                        server_tools.insert(tool_id.clone(), vec![default_tool]);
                        info!("Added default tool for server {} after timeout", tool_id);
                    }
                }
            }
            Err(e) => {
                error!("Failed to spawn process for {}: {}", tool_id, e);
                return Ok(ToolRegistrationResponse {
                    success: false,
                    message: format!("Tool registered but failed to start process: {}", e),
                    tool_id: Some(tool_id),
                });
            }
        }

        info!("Tool registration completed for: {}", request.tool_name);
        Ok(ToolRegistrationResponse {
            success: true,
            message: format!("Tool '{}' registered successfully", request.tool_name),
            tool_id: Some(tool_id),
        })
    }

    /// List all registered tools
    async fn list_tools(&self) -> Result<Vec<Value>, String> {
        let mcp_state = self.mcp_state.read().await;
        let registry = mcp_state.tool_registry.read().await;
        let mut tools = Vec::new();

        // Get all tools from the registry
        let tool_map = registry.get_all_tools()?;

        for (id, tool_struct) in tool_map {
            // Convert the Tool struct to a Value
            let mut tool_value = json!({
                "name": tool_struct.name,
                "description": tool_struct.description,
                "enabled": tool_struct.enabled,
                "tool_type": tool_struct.tool_type,
                "id": id,
            });

            // Add optional fields if they exist
            if let Some(entry_point) = &tool_struct.entry_point {
                if let Some(obj) = tool_value.as_object_mut() {
                    obj.insert("entry_point".to_string(), json!(entry_point));
                }
            }

            if let Some(configuration) = &tool_struct.configuration {
                if let Some(obj) = tool_value.as_object_mut() {
                    obj.insert(
                        "configuration".to_string(),
                        json!({
                            "command": configuration.command,
                            "args": configuration.args,
                            "env": configuration.env,
                        }),
                    );
                }
            }

            if let Some(distribution) = &tool_struct.distribution {
                if let Some(obj) = tool_value.as_object_mut() {
                    obj.insert(
                        "distribution".to_string(),
                        serde_json::to_value(distribution).unwrap_or(serde_json::Value::Null),
                    );
                }
            }

            // Add process status - check the processes map
            if let Some(obj) = tool_value.as_object_mut() {
                let process_running = {
                    let process_manager = mcp_state.process_manager.read().await;
                    process_manager.processes.contains_key(&id)
                };
                obj.insert("process_running".to_string(), json!(process_running));

                // Add number of available tools from this server
                let server_tool_count = {
                    let server_tools = mcp_state.server_tools.read().await;
                    server_tools.get(&id).map_or_else(|| 0, |tools| tools.len())
                };
                obj.insert("tool_count".to_string(), json!(server_tool_count));
            }

            tools.push(tool_value);
        }
        println!("tools: {:?}", tools);
        Ok(tools)
    }

    /// List all available tools from all running MCP servers
    async fn list_all_server_tools(&self) -> Result<Vec<Value>, String> {
        let mcp_state = self.mcp_state.read().await;
        let server_tools = mcp_state.server_tools.read().await;
        let mut all_tools = Vec::new();

        for (server_id, tools) in &*server_tools {
            for tool in tools {
                // Extract the basic tool information
                let mut tool_info = serde_json::Map::new();

                // Copy the original tool properties
                if let Some(obj) = tool.as_object() {
                    for (key, value) in obj {
                        tool_info.insert(key.clone(), value.clone());
                    }
                }

                // Add server_id
                tool_info.insert("server_id".to_string(), json!(server_id));

                // Create a proxy ID
                let tool_id = tool.get("id").and_then(|v| v.as_str()).unwrap_or("");
                let proxy_id = format!("{}:{}", server_id, tool_id);
                tool_info.insert("proxy_id".to_string(), json!(proxy_id));

                all_tools.push(json!(tool_info));
            }
        }

        Ok(all_tools)
    }

    /// Discover tools from a specific MCP server
    async fn discover_tools(
        &self,
        request: DiscoverServerToolsRequest,
    ) -> Result<DiscoverServerToolsResponse, String> {
        let mcp_state = self.mcp_state.read().await;
        // Check if the server exists and is running
        let server_running = {
            let process_manager = mcp_state.process_manager.read().await;
            process_manager
                .processes
                .get(&request.server_id)
                .is_some_and(|p| p.is_some())
        };

        if !server_running {
            return Ok(DiscoverServerToolsResponse {
                success: false,
                tools: None,
                error: Some(format!(
                    "Server with ID '{}' is not running",
                    request.server_id
                )),
            });
        }

        let mcp_state = self.mcp_state.read().await;
        let mut process_manager = mcp_state.process_manager.write().await;
        // Discover tools from the server
        let result = if let Some((stdin, stdout)) =
            process_manager.process_ios.get_mut(&request.server_id)
        {
            discover_server_tools(&request.server_id, stdin, stdout).await
        } else {
            Err(format!(
                "Server {} not found or not running",
                request.server_id
            ))
        };

        // Release the process_manager lock before accessing server_tools
        drop(process_manager);

        {
            let mcp_state = self.mcp_state.read().await;
            let mut server_tools = mcp_state.server_tools.write().await;
            // Get a write lock on server_tools to update
            match result {
                Ok(tools) => {
                    // Store the discovered tools
                    server_tools.insert(request.server_id.clone(), tools.clone());

                    Ok(DiscoverServerToolsResponse {
                        success: true,
                        tools: Some(tools),
                        error: None,
                    })
                }
                Err(e) => Ok(DiscoverServerToolsResponse {
                    success: false,
                    tools: None,
                    error: Some(format!("Failed to discover tools: {}", e)),
                }),
            }
        }
    }

    /// Execute a tool from an MCP server
    async fn execute_proxy_tool(
        &self,
        request: ToolExecutionRequest,
    ) -> Result<ToolExecutionResponse, String> {
        let mcp_state = self.mcp_state.read().await;
        // Extract server_id and tool_id from the proxy_id
        let parts: Vec<&str> = request.tool_id.split(':').collect();
        println!("parts: {:?}", parts);
        if parts.len() != 2 {
            return Err("Invalid tool_id format. Expected 'server_id:tool_id'".to_string());
        }

        let server_id = parts[0];
        println!("server_id: {}", server_id);
        let tool_id = parts[1];
        println!("tool_id: {}", tool_id);

        // Execute the tool on the server
        let mut process_manager = mcp_state.process_manager.write().await;

        // Check if the server exists
        let result = if !process_manager.process_ios.contains_key(server_id) {
            Err(MCPError::ServerNotFound(server_id.to_string()))
        } else {
            // Get stdin/stdout for the server
            let (stdin, stdout) = process_manager.process_ios.get_mut(server_id).unwrap();

            // Execute the tool

            execute_server_tool(
                server_id,
                tool_id,
                request.parameters.clone(),
                stdin,
                stdout,
            )
            .await
        };

        // Release the lock
        drop(process_manager);

        match result {
            Ok(result) => Ok(ToolExecutionResponse {
                success: true,
                result: Some(result),
                error: None,
            }),
            Err(e) => Ok(ToolExecutionResponse {
                success: false,
                result: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Update a tool's status (enabled/disabled)
    async fn update_tool_status(
        &self,
        request: ToolUpdateRequest,
    ) -> Result<ToolUpdateResponse, String> {
        let mcp_state = self.mcp_state.read().await;
        // First, check if the tool exists and get the necessary information
        let tool_info = {
            let registry = mcp_state.tool_registry.read().await;

            if let Ok(tool) = registry.get_tool(&request.tool_id) {
                // Extract and clone the necessary values
                let tool_type = tool.tool_type.clone();
                let entry_point = tool.entry_point.clone().unwrap_or_default();
                let process_running = {
                    let process_manager = mcp_state.process_manager.read().await;
                    process_manager
                        .processes
                        .get(&request.tool_id)
                        .is_some_and(|p| p.is_some())
                };

                Some((tool_type, entry_point, process_running))
            } else {
                None
            }
        };

        // If the tool doesn't exist, return an error
        if tool_info.is_none() {
            return Ok(ToolUpdateResponse {
                success: false,
                message: format!("Tool with ID '{}' not found", request.tool_id),
            });
        }

        // Now handle the process based on the enabled status
        let result = {
            let registry = mcp_state.tool_registry.write().await;

            // Get the current tool data
            let mut tool = registry.get_tool(&request.tool_id)?;

            // Update the enabled status
            tool.enabled = request.enabled;

            // Save the updated tool
            registry.save_tool(&request.tool_id, &tool)?;

            // Drop the write lock before trying to restart the tool
            drop(registry);

            if request.enabled {
                mcp_state.restart_tool(&request.tool_id).await
            } else {
                Ok(())
            }
        };

        // Handle any errors from the process management
        if let Err(e) = result {
            return Ok(ToolUpdateResponse {
                success: false,
                message: e,
            });
        }

        // Return success
        Ok(ToolUpdateResponse {
            success: true,
            message: format!(
                "Tool '{}' status updated to {}",
                request.tool_id,
                if request.enabled {
                    "enabled"
                } else {
                    "disabled"
                }
            ),
        })
    }

    /// Update a tool's configuration (environment variables)
    async fn update_tool_config(
        &self,
        request: ToolConfigUpdateRequest,
    ) -> Result<ToolConfigUpdateResponse, String> {
        let mcp_state = self.mcp_state.read().await;
        info!("Updating configuration for tool: {}", request.tool_id);

        // First, check if the tool exists
        let (tool_exists, is_enabled) = {
            let registry = mcp_state.tool_registry.read().await;
            match registry.get_tool(&request.tool_id) {
                Ok(tool) => (true, tool.enabled),
                Err(_) => (false, false),
            }
        };

        // If the tool doesn't exist, return an error
        if !tool_exists {
            error!("Tool with ID '{}' not found", request.tool_id);
            return Ok(ToolConfigUpdateResponse {
                success: false,
                message: format!("Tool with ID '{}' not found", request.tool_id),
            });
        }

        info!("Tool '{}' found, enabled: {}", request.tool_id, is_enabled);

        // Update the tool configuration
        let registry = mcp_state.tool_registry.write().await;

        // Get the current tool data
        let mut tool = registry.get_tool(&request.tool_id)?;

        // Create or update the configuration object
        if tool.configuration.is_none() {
            tool.configuration = Some(ToolConfiguration {
                command: None,
                args: None,
                env: Some(HashMap::new()),
            });
        }

        if let Some(configuration) = &mut tool.configuration {
            // Create or update the env object
            if configuration.env.is_none() {
                configuration.env = Some(HashMap::new());
            }

            if let Some(env_map) = &mut configuration.env {
                // Update each environment variable from the config HashMap
                for (key, value) in &request.config {
                    info!(
                        "Setting environment variable for tool {}: {}={}",
                        request.tool_id, key, value
                    );
                    // Convert to ToolEnvironment
                    env_map.insert(
                        key.clone(),
                        ToolEnvironment {
                            description: "".to_string(),
                            default: Some(value.clone()),
                            required: false,
                        },
                    );
                }
            }
        }

        // Save the updated tool
        registry.save_tool(&request.tool_id, &tool)?;

        // Return success
        Ok(ToolConfigUpdateResponse {
            success: true,
            message: format!("Tool '{}' configuration updated", request.tool_id),
        })
    }

    /// Uninstall a registered tool
    async fn uninstall_tool(
        &self,
        request: ToolUninstallRequest,
    ) -> Result<ToolUninstallResponse, String> {
        let mcp_state = self.mcp_state.read().await;
        let registry = mcp_state.tool_registry.write().await;

        // First check if the tool exists
        if registry.get_tool(&request.tool_id).is_err() {
            return Ok(ToolUninstallResponse {
                success: false,
                message: format!("Tool with ID '{}' not found", request.tool_id),
            });
        }

        // Kill the process if it's running
        let mut process_manager = mcp_state.process_manager.write().await;
        if let Some(Some(process)) = process_manager.processes.get_mut(&request.tool_id) {
            if let Err(e) = kill_process(process).await {
                return Ok(ToolUninstallResponse {
                    success: false,
                    message: format!("Failed to kill process: {}", e),
                });
            }
        }

        // Remove the process and IOs from the process manager
        process_manager.processes.remove(&request.tool_id);
        process_manager.process_ios.remove(&request.tool_id);
        drop(process_manager);

        // Remove server tools
        let mut server_tools = mcp_state.server_tools.write().await;
        server_tools.remove(&request.tool_id);

        // Delete the tool using registry's delete_tool method
        if let Err(e) = registry.delete_tool(&request.tool_id) {
            return Ok(ToolUninstallResponse {
                success: false,
                message: format!("Failed to delete tool: {}", e),
            });
        }

        Ok(ToolUninstallResponse {
            success: true,
            message: "Tool uninstalled successfully".to_string(),
        })
    }

    /// Get all server data in a single function to avoid multiple locks
    async fn get_all_server_data(&self) -> Result<Value, String> {
        let mcp_state = self.mcp_state.read().await;
        // Get registry data
        let registry = mcp_state.tool_registry.read().await;
        let process_manager = mcp_state.process_manager.read().await;
        let server_tools = mcp_state.server_tools.read().await;

        // 1. Get registered servers
        let mut servers = Vec::new();
        let tool_map = registry.get_all_tools()?;

        for (id, tool_struct) in tool_map {
            // Convert the Tool struct to a Value
            let mut tool_value = json!({
                "name": tool_struct.name,
                "description": tool_struct.description,
                "enabled": tool_struct.enabled,
                "tool_type": tool_struct.tool_type,
                "id": id,
            });

            // Add optional fields if they exist
            if let Some(entry_point) = &tool_struct.entry_point {
                if let Some(obj) = tool_value.as_object_mut() {
                    obj.insert("entry_point".to_string(), json!(entry_point));
                }
            }

            if let Some(configuration) = &tool_struct.configuration {
                if let Some(obj) = tool_value.as_object_mut() {
                    obj.insert(
                        "configuration".to_string(),
                        json!({
                            "command": configuration.command,
                            "args": configuration.args
                        }),
                    );
                }
            }

            if let Some(distribution) = &tool_struct.distribution {
                if let Some(obj) = tool_value.as_object_mut() {
                    obj.insert(
                        "distribution".to_string(),
                        serde_json::to_value(distribution).unwrap_or(serde_json::Value::Null),
                    );
                }
            }

            // Add process status - check the processes map
            if let Some(obj) = tool_value.as_object_mut() {
                let process_running = process_manager.processes.contains_key(&id);
                obj.insert("process_running".to_string(), json!(process_running));

                // Add number of available tools from this server
                let server_tool_count =
                    server_tools.get(&id).map_or_else(|| 0, |tools| tools.len());
                obj.insert("tool_count".to_string(), json!(server_tool_count));
            }

            servers.push(tool_value);
        }

        // 2. Get all server tools
        let mut all_tools = Vec::new();
        for (server_id, tools) in &*server_tools {
            for tool in tools {
                // Extract the basic tool information
                let mut tool_info = serde_json::Map::new();

                // Copy the original tool properties
                if let Some(obj) = tool.as_object() {
                    for (key, value) in obj {
                        tool_info.insert(key.clone(), value.clone());
                    }
                }

                // Add server_id
                tool_info.insert("server_id".to_string(), json!(server_id));

                // Create a proxy ID
                let tool_id = tool.get("id").and_then(|v| v.as_str()).unwrap_or("");
                let proxy_id = format!("{}:{}", server_id, tool_id);
                tool_info.insert("proxy_id".to_string(), json!(proxy_id));

                all_tools.push(json!(tool_info));
            }
        }

        // Return all data in one response
        Ok(json!({
            "servers": servers,
            "tools": all_tools
        }))
    }

    /// Restart a tool by its ID
    async fn restart_tool_command(&self, tool_id: String) -> Result<ToolUpdateResponse, String> {
        let mcp_state = self.mcp_state.read().await;
        info!("Received request to restart tool: {}", tool_id);

        // Check if the tool exists
        let tool_exists = {
            let registry = mcp_state.tool_registry.read().await;
            registry.get_tool(&tool_id).is_ok()
        };

        if !tool_exists {
            error!("Tool with ID '{}' not found for restart", tool_id);
            return Ok(ToolUpdateResponse {
                success: false,
                message: format!("Tool with ID '{}' not found", tool_id),
            });
        }

        info!("Tool '{}' found, attempting to restart", tool_id);

        // Restart the tool using MCPState
        let restart_result = mcp_state.restart_tool(&tool_id).await;

        match restart_result {
            Ok(_) => {
                info!("Successfully restarted tool: {}", tool_id);
                Ok(ToolUpdateResponse {
                    success: true,
                    message: format!("Tool '{}' restarted successfully", tool_id),
                })
            }
            Err(e) => {
                error!("Failed to restart tool {}: {}", tool_id, e);
                Ok(ToolUpdateResponse {
                    success: false,
                    message: format!("Failed to restart tool: {}", e),
                })
            }
        }
    }

    /// Initialize the MCP server and start background services
    async fn init_mcp_server(&self) -> Result<()> {
        info!("Starting background initialization of MCP services");

        // Get all tools from database
        let tools = match self.tool_registry.read().await.get_all_tools() {
            Ok(tools) => tools,
            Err(e) => {
                error!("Failed to get tools from database: {}", e);
                return Err(anyhow::anyhow!("Failed to get tools from database: {}", e));
            }
        };

        // Update the state with the new registry
        // Create a vector of futures for parallel execution
        let mut restart_futures = Vec::new();

        // Prepare restart tasks for all enabled tools
        for (tool_id_str, metadata) in tools {
            if metadata.enabled {
                info!("Found enabled tool: {}", tool_id_str);
                let tool_id = tool_id_str.clone();
                let mcp_state_arc_clone = self.mcp_state.clone();

                // Create a future for each tool restart
                let restart_future = async move {
                    let mcp_state_clone_write_guard = mcp_state_arc_clone.write().await;
                    match mcp_state_clone_write_guard.restart_tool(&tool_id).await {
                        Ok(()) => {
                            info!("Successfully spawned process for tool: {}", tool_id);
                        }
                        Err(e) => {
                            error!("Failed to spawn process for tool {}: {}", tool_id, e);
                        }
                    }

                    // Return the tool_id for logging purposes
                    tool_id
                };

                restart_futures.push(restart_future);
            }
        }

        // Execute all restart tasks in parallel
        if !restart_futures.is_empty() {
            info!(
                "Starting parallel initialization of {} tools",
                restart_futures.len()
            );
            let results = future::join_all(restart_futures).await;
            info!(
                "Completed parallel initialization of {} tools",
                results.len()
            );
        } else {
            info!("No enabled tools found to initialize");
        }

        // Start the process monitor
        let mcp_state_clone = self.mcp_state.clone();
        mcp_state_clone.start_process_monitor().await;

        Ok(())
    }

    /// Kill all running processes
    async fn kill_all_processes(&self) -> Result<()> {
        let mcp_state = self.mcp_state.read().await;
        mcp_state.kill_all_processes().await;
        Ok(())
    }
}
