use crate::mcp_protocol::{discover_server_tools, execute_server_tool};
use crate::mcp_state::mcp_state::McpStateProcessMonitor;
use crate::mcp_state::mcp_state_process_utils::{kill_process, spawn_process};
use crate::models::types::{
    DiscoverServerToolsRequest, Distribution, RuntimeServer, ServerConfigUpdateRequest, ServerConfiguration,
    ServerDefinition, ServerEnvironment, ServerId, ServerRegistrationRequest,
    ServerRegistrationResponse, ServerToolInfo, ServerUninstallResponse, ServerUpdateRequest,
    ToolConfigUpdateResponse, ToolExecutionRequest, ToolExecutionResponse, ToolUninstallRequest,
    ToolUpdateResponse,
};
use crate::utils::github::{GitHubRepo, parse_github_url, fetch_github_file, extract_env_vars_from_readme};
use crate::MCPError;
use anyhow::Result;
use async_trait::async_trait;
use futures::future;
use log::{error, info};
use reqwest::Client;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use tokio::time::Duration;
use toml::Table;

use super::mcp_core::MCPCore;

#[async_trait]
pub trait McpCoreProxyExt {
    async fn register_server(
        &self,
        tool: ServerRegistrationRequest,
    ) -> Result<ServerRegistrationResponse, String>;
    async fn list_servers(&self) -> Result<Vec<RuntimeServer>, String>;
    async fn list_all_server_tools(&self) -> Result<Vec<ServerToolInfo>, String>;
    async fn list_server_tools(
        &self,
        request: DiscoverServerToolsRequest,
    ) -> Result<Vec<ServerToolInfo>, String>;
    async fn execute_proxy_tool(
        &self,
        request: ToolExecutionRequest,
    ) -> Result<ToolExecutionResponse, String>;
    async fn update_server_status(
        &self,
        request: ServerUpdateRequest,
    ) -> Result<ToolUpdateResponse, String>;
    async fn update_server_config(
        &self,
        request: ServerConfigUpdateRequest,
    ) -> Result<ToolConfigUpdateResponse, String>;
    async fn uninstall_server(
        &self,
        request: ToolUninstallRequest,
    ) -> Result<ServerUninstallResponse, String>;
    async fn restart_server_command(&self, tool_id: String) -> Result<ToolUpdateResponse, String>;
    async fn init_mcp_server(&self) -> Result<()>;
    async fn kill_all_processes(&self) -> Result<()>;
    /// Import a server from a GitHub repository URL
    async fn import_server_from_url(&self, github_url: String) -> Result<ServerRegistrationResponse, String>;
    /// Process a Node.js project from package.json content
    async fn process_nodejs_project(&self, package_json_content: String, repo_info: &GitHubRepo, env_vars: HashSet<String>) -> Result<ServerRegistrationResponse, String>;
    /// Process a Python project from pyproject.toml content
    async fn process_python_project(&self, pyproject_toml_content: String, repo_info: &GitHubRepo, env_vars: HashSet<String>) -> Result<ServerRegistrationResponse, String>;
}

#[async_trait]
impl McpCoreProxyExt for MCPCore {
    /// Register a new tool with the MCP server
    async fn register_server(
        &self,
        request: ServerRegistrationRequest,
    ) -> Result<ServerRegistrationResponse, String> {
        // Log configuration details if present
        if let Some(config) = &request.configuration {
            if let Some(cmd) = &config.command {
                info!("Command: {}", cmd);
            } else {
                info!("Command: Not specified in configuration");
            }
        } else {
            info!("Configuration not provided");
        }
        let registry = self.tool_registry.write().await;

        // Generate a simple tool ID (in production, use UUIDs)
        let server_id = request.server_id.clone();
        info!("Generated server ID: {}", server_id);

        // Create the Tool struct
        let server = ServerDefinition {
            name: request.server_name.clone(),
            description: request.description.clone(),
            enabled: true, // Default to enabled
            tools_type: request.tools_type.clone(),
            entry_point: None,
            configuration: request.configuration,
            distribution: request.distribution,
        };

        // Save the tool in the registry
        registry.save_server(&server_id, &server)?;
        drop(registry);

        let mcp_state_clone = self.mcp_state.clone();
        {
            // Create a default empty tools list
            let mcp_state = mcp_state_clone.write().await;
            let mut server_tools = mcp_state.server_tools.write().await;
            server_tools.insert(server_id.clone(), Vec::new());
            
            // Send notification that tools have changed (empty list)
            let server_id_clone = server_id.clone();
            tokio::spawn(async move {
                crate::utils::http_client::notify_tools_changed(
                    &server_id_clone,
                    Vec::new(),
                    Vec::new(),
                    Vec::new()
                ).await;
            });
        }

        // Extract environment variables from the tool configuration
        let env_vars = if let Some(configuration) = &server.configuration {
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
        let config_value = if let Some(configuration) = &server.configuration {
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
            &server_id,
            &request.tools_type,
            env_vars.as_ref(),
        )
        .await;

        let mcp_state_clone = self.mcp_state.clone();
        match spawn_result {
            Ok((process, stdin, stdout)) => {
                info!("Process spawned successfully for tool ID: {}", server_id);
                {
                    let mcp_state = mcp_state_clone.write().await;
                    let mut process_manager = mcp_state.process_manager.write().await;
                    process_manager
                        .processes
                        .insert(server_id.clone(), Some(process));
                    process_manager
                        .process_ios
                        .insert(server_id.clone(), (stdin, stdout));
                }
                // Wait a moment for the server to start
                info!("Waiting for server to initialize...");
                tokio::time::sleep(Duration::from_secs(3)).await;

                // Try to discover tools from this server with a timeout to avoid hanging
                info!("Attempting to discover tools from server {}", server_id);
                let discover_result = tokio::time::timeout(Duration::from_secs(15), async {
                    let mcp_state = mcp_state_clone.write().await;
                    let mut process_manager = mcp_state.process_manager.write().await;
                    if let Some((stdin, stdout)) = process_manager.process_ios.get_mut(&server_id) {
                        discover_server_tools(&server_id, stdin, stdout).await
                    } else {
                        Err(format!("Server {} not found or not running", server_id))
                    }
                })
                .await;

                // Handle the result of the discovery attempt
                match discover_result {
                    Ok(Ok(tools)) => {
                        info!(
                            "Successfully discovered {} tools from {}",
                            tools.len(),
                            server_id
                        );
                        let mcp_state = mcp_state_clone.write().await;
                        let mut server_tools = mcp_state.server_tools.write().await;
                        // Get tool names for notification
                        let tool_names: Vec<String> = tools.iter().map(|t| t.name.clone()).collect();
                        server_tools.insert(server_id.clone(), tools);
                        
                        // Send notification that tools have changed
                        let server_id_clone = server_id.clone();
                        tokio::spawn(async move {
                            crate::utils::http_client::notify_tools_changed(
                                &server_id_clone,
                                tool_names,
                                Vec::new(),
                                Vec::new()
                            ).await;
                        });
                    }
                    Ok(Err(e)) => {
                        error!("Error discovering tools from server {}: {}", server_id, e);
                    }
                    Err(_) => {
                        error!("Timeout while discovering tools from server {}", server_id);
                        info!("Added default tool for server {} after timeout", server_id);
                    }
                }
            }
            Err(e) => {
                error!("Failed to spawn process for {}: {}", server_id, e);
                return Ok(ServerRegistrationResponse {
                    success: false,
                    message: format!("Tool registered but failed to start process: {}", e),
                    tool_id: Some(server_id),
                });
            }
        }

        info!("Tool registration completed for: {}", request.server_name);
        Ok(ServerRegistrationResponse {
            success: true,
            message: format!("Tool '{}' registered successfully", request.server_name),
            tool_id: Some(server_id),
        })
    }

    /// List all registered tools
    async fn list_servers(&self) -> Result<Vec<RuntimeServer>, String> {
        let mcp_state = self.mcp_state.read().await;
        let registry = mcp_state.tool_registry.read().await;

        let tool_map = registry.get_all_servers()?;
        let mut tools = Vec::new();

        for (id, tool_struct) in tool_map {
            let process_running = {
                let process_manager = mcp_state.process_manager.read().await;
                process_manager.processes.contains_key(&id)
            };

            let tool_count = {
                let server_tools = mcp_state.server_tools.read().await;
                server_tools.get(&id).map_or(0, |tools| tools.len())
            };

            tools.push(RuntimeServer {
                definition: tool_struct,
                id: ServerId::new(id),
                process_running,
                tool_count,
            });
        }

        Ok(tools)
    }

    /// List all available tools from all running MCP servers
    async fn list_all_server_tools(&self) -> Result<Vec<ServerToolInfo>, String> {
        let mcp_state = self.mcp_state.read().await;
        let server_tools = mcp_state.server_tools.read().await;
        let mut all_tools = Vec::new();

        for tools in (*server_tools).values() {
            all_tools.extend(tools.iter().cloned());
        }
        Ok(all_tools)
    }

    /// Discover tools from a specific MCP server
    async fn list_server_tools(
        &self,
        request: DiscoverServerToolsRequest,
    ) -> Result<Vec<ServerToolInfo>, String> {
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
            return Err(format!(
                "Server with ID '{}' is not running",
                request.server_id
            ));
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
                    // Store the discovered tools and return them
                    // Get tool names for notification
                    let tool_names: Vec<String> = tools.iter().map(|t| t.name.clone()).collect();
                    server_tools.insert(request.server_id.clone(), tools.clone());
                    
                    // Send notification that tools have changed
                    let server_id_clone = request.server_id.clone();
                    tokio::spawn(async move {
                        crate::utils::http_client::notify_tools_changed(
                            &server_id_clone,
                            tool_names,
                            Vec::new(),
                            Vec::new()
                        ).await;
                    });
                    Ok(tools)
                }
                Err(e) => Err(format!("Failed to discover tools: {}", e)),
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
        let tool_id = parts[1];

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
    async fn update_server_status(
        &self,
        request: ServerUpdateRequest,
    ) -> Result<ToolUpdateResponse, String> {
        let mcp_state = self.mcp_state.read().await;
        // First, check if the tool exists and get the necessary information
        let tool_info = {
            let registry = mcp_state.tool_registry.read().await;

            if let Ok(tool) = registry.get_server(&request.server_id) {
                // Extract and clone the necessary values
                let tools_type = tool.tools_type.clone();
                let entry_point = tool.entry_point.clone().unwrap_or_default();
                let process_running = {
                    let process_manager = mcp_state.process_manager.read().await;
                    process_manager
                        .processes
                        .get(&request.server_id)
                        .is_some_and(|p| p.is_some())
                };

                Some((tools_type, entry_point, process_running))
            } else {
                None
            }
        };

        // If the tool doesn't exist, return an error
        if tool_info.is_none() {
            return Ok(ToolUpdateResponse {
                success: false,
                message: format!("Tool with ID '{}' not found", request.server_id),
            });
        }

        // Now handle the process based on the enabled status
        let result = {
            let registry = mcp_state.tool_registry.write().await;

            // Get the current tool data
            let mut tool = registry.get_server(&request.server_id)?;

            // Update the enabled status
            tool.enabled = request.enabled;

            // Save the updated tool
            registry.save_server(&request.server_id, &tool)?;

            // Drop the write lock before trying to restart the tool
            drop(registry);

            if request.enabled {
                mcp_state.restart_server(&request.server_id).await
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
                request.server_id,
                if request.enabled {
                    "enabled"
                } else {
                    "disabled"
                }
            ),
        })
    }

    /// Update a tool's configuration (environment variables)
    async fn update_server_config(
        &self,
        request: ServerConfigUpdateRequest,
    ) -> Result<ToolConfigUpdateResponse, String> {
        let mcp_state = self.mcp_state.read().await;
        info!("Updating configuration for tool: {}", request.server_id);

        // First, check if the tool exists
        let (tool_exists, is_enabled) = {
            let registry = mcp_state.tool_registry.read().await;
            match registry.get_server(&request.server_id) {
                Ok(tool) => (true, tool.enabled),
                Err(_) => (false, false),
            }
        };

        // If the tool doesn't exist, return an error
        if !tool_exists {
            error!("Tool with ID '{}' not found", request.server_id);
            return Ok(ToolConfigUpdateResponse {
                success: false,
                message: format!("Tool with ID '{}' not found", request.server_id),
            });
        }

        info!(
            "Tool '{}' found, enabled: {}",
            request.server_id, is_enabled
        );

        // Update the tool configuration
        let registry = mcp_state.tool_registry.write().await;

        // Get the current tool data
        let mut tool = registry.get_server(&request.server_id)?;

        // Create or update the configuration object
        if tool.configuration.is_none() {
            tool.configuration = Some(ServerConfiguration {
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
                        request.server_id, key, value
                    );
                    // Convert to ToolEnvironment
                    env_map.insert(
                        key.clone(),
                        ServerEnvironment {
                            description: "".to_string(),
                            default: Some(value.clone()),
                            required: false,
                        },
                    );
                }
            }
        }

        // Save the updated tool
        registry.save_server(&request.server_id, &tool)?;

        // Return success
        Ok(ToolConfigUpdateResponse {
            success: true,
            message: format!("Tool '{}' configuration updated", request.server_id),
        })
    }

    /// Uninstall a registered tool
    async fn uninstall_server(
        &self,
        request: ToolUninstallRequest,
    ) -> Result<ServerUninstallResponse, String> {
        let mcp_state = self.mcp_state.read().await;
        let registry = mcp_state.tool_registry.write().await;

        // First check if the tool exists
        if registry.get_server(&request.server_id).is_err() {
            return Ok(ServerUninstallResponse {
                success: false,
                message: format!("Tool with ID '{}' not found", request.server_id),
            });
        }

        // Kill the process if it's running
        let mut process_manager = mcp_state.process_manager.write().await;
        if let Some(Some(process)) = process_manager.processes.get_mut(&request.server_id) {
            if let Err(e) = kill_process(process).await {
                return Ok(ServerUninstallResponse {
                    success: false,
                    message: format!("Failed to kill process: {}", e),
                });
            }
        }

        // Remove the process and IOs from the process manager
        process_manager.processes.remove(&request.server_id);
        process_manager.process_ios.remove(&request.server_id);
        drop(process_manager);

        // Remove server tools
        let mut server_tools = mcp_state.server_tools.write().await;
        server_tools.remove(&request.server_id);

        // Delete the tool using registry's delete_tool method
        if let Err(e) = registry.delete_server(&request.server_id) {
            return Ok(ServerUninstallResponse {
                success: false,
                message: format!("Failed to delete tool: {}", e),
            });
        }

        Ok(ServerUninstallResponse {
            success: true,
            message: "Tool uninstalled successfully".to_string(),
        })
    }

    /// Restart a server by its ID
    async fn restart_server_command(
        &self,
        server_id: String,
    ) -> Result<ToolUpdateResponse, String> {
        let mcp_state = self.mcp_state.read().await;
        info!("Received request to restart tool: {}", server_id);

        // Check if the tool exists
        let tool_exists = {
            let registry = mcp_state.tool_registry.read().await;
            registry.get_server(&server_id).is_ok()
        };

        if !tool_exists {
            error!("Tool with ID '{}' not found for restart", server_id);
            return Ok(ToolUpdateResponse {
                success: false,
                message: format!("Tool with ID '{}' not found", server_id),
            });
        }

        info!("Tool '{}' found, attempting to restart", server_id);

        // Restart the tool using MCPState
        let restart_result = mcp_state.restart_server(&server_id).await;

        match restart_result {
            Ok(_) => {
                info!("Successfully restarted tool: {}", server_id);
                Ok(ToolUpdateResponse {
                    success: true,
                    message: format!("Tool '{}' restarted successfully", server_id),
                })
            }
            Err(e) => {
                error!("Failed to restart tool {}: {}", server_id, e);
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
        let tools = match self.tool_registry.read().await.get_all_servers() {
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
                    match mcp_state_clone_write_guard.restart_server(&tool_id).await {
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

    /// Import a server from a GitHub repository URL
    async fn import_server_from_url(&self, github_url: String) -> Result<ServerRegistrationResponse, String> {
        info!("Importing server from URL: {}", github_url);
        
        // Parse GitHub URL
        let repo_info = parse_github_url(&github_url)?;
        info!("Parsed GitHub URL: owner={}, repo={}", repo_info.owner, repo_info.repo);
        
        // Create HTTP client
        let client = Client::builder()
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
        
        // Try to fetch README.md to extract environment variables
        let mut env_vars = HashSet::new();
        let readme_result = fetch_github_file(&client, &repo_info.owner, &repo_info.repo, "README.md").await;
        
        if let Ok(readme_content) = readme_result {
            info!("Found README.md, extracting environment variables");
            env_vars = extract_env_vars_from_readme(&readme_content);
            info!("Extracted {} environment variables from README.md", env_vars.len());
            
            if !env_vars.is_empty() {
                info!("Extracted environment variables: {:?}", env_vars);
            }
        } else {
            info!("README.md not found or could not be parsed");
        }
        
        // Try to fetch package.json first (Node.js project)
        let package_json_result = fetch_github_file(&client, &repo_info.owner, &repo_info.repo, "package.json").await;
        
        if let Ok(package_json_content) = package_json_result {
            info!("Found package.json, processing as Node.js project");
            return self.process_nodejs_project(package_json_content, &repo_info, env_vars).await;
        }
        
        // If package.json not found, try pyproject.toml (Python project)
        let pyproject_toml_result = fetch_github_file(&client, &repo_info.owner, &repo_info.repo, "pyproject.toml").await;
        
        if let Ok(pyproject_toml_content) = pyproject_toml_result {
            info!("Found pyproject.toml, processing as Python project");
            return self.process_python_project(pyproject_toml_content, &repo_info, env_vars).await;
        }
        
        // If neither found, return error
        Err(format!("Could not find package.json or pyproject.toml in repository {}/{}", repo_info.owner, repo_info.repo))
    }
    
    /// Process a Node.js project from package.json content
    async fn process_nodejs_project(&self, package_json_content: String, repo_info: &GitHubRepo, env_vars: HashSet<String>) -> Result<ServerRegistrationResponse, String> {
        // Parse package.json
        let package_json: Value = serde_json::from_str(&package_json_content)
            .map_err(|e| format!("Failed to parse package.json: {}", e))?;
        
        // Extract package name
        let package_name = package_json.get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'name' field in package.json".to_string())?
            .to_string();
        
        // Extract description
        let description = package_json.get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("MCP Server imported from GitHub")
            .to_string();
        
        // Create server ID from owner/repo
        let server_id = format!("{}/{}", repo_info.owner, repo_info.repo);
        
        // Create server name from package name
        let server_name = format!("{} MCP Server", package_name);
        
        // Create environment variables map from extracted env vars
        let mut env_map = HashMap::new();
        for var_name in env_vars {
            env_map.insert(
                var_name.clone(),
                ServerEnvironment {
                    description: format!("Extracted from README.md: {}", var_name),
                    default: Some("".to_string()), // Empty default value
                    required: true,
                },
            );
        }
        
        // Create configuration
        let configuration = Some(ServerConfiguration {
            command: Some("npx".to_string()),
            args: Some(vec!["-y".to_string(), package_name.clone()]),
            env: Some(env_map),
        });
        
        // Create distribution
        let distribution = Some(Distribution {
            r#type: "npm".to_string(),
            package: package_name,
        });
        
        // Create registration request
        let request = ServerRegistrationRequest {
            server_id,
            server_name,
            description,
            tools_type: "node".to_string(),
            configuration,
            distribution,
        };
        
        // Register the server
        self.register_server(request).await
    }
    
    /// Process a Python project from pyproject.toml content
    async fn process_python_project(&self, pyproject_toml_content: String, repo_info: &GitHubRepo, env_vars: HashSet<String>) -> Result<ServerRegistrationResponse, String> {
        // Parse pyproject.toml
        let pyproject_toml: Table = pyproject_toml_content.parse::<Table>()
            .map_err(|e| format!("Failed to parse pyproject.toml: {}", e))?;
        
        // Extract package name
        let project = pyproject_toml.get("project")
            .ok_or_else(|| "Missing 'project' section in pyproject.toml".to_string())?
            .as_table()
            .ok_or_else(|| "Invalid 'project' section in pyproject.toml".to_string())?;
        
        let package_name = project.get("name")
            .ok_or_else(|| "Missing 'name' field in pyproject.toml".to_string())?
            .as_str()
            .ok_or_else(|| "Invalid 'name' field in pyproject.toml".to_string())?
            .to_string();
        
        // Extract description
        let description = project.get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("MCP Server imported from GitHub")
            .to_string();
        
        // Check for project.scripts section to determine entry point
        let entry_point = if let Some(scripts) = project.get("scripts")
            .and_then(|v| v.as_table()) {
            // Use the first script as entry point
            if !scripts.is_empty() {
                let script_name = scripts.keys().next().unwrap();
                Some(script_name.to_string())
            } else {
                None
            }
        } else {
            None
        };
        
        // Create server ID from owner/repo
        let server_id = format!("{}/{}", repo_info.owner, repo_info.repo);
        
        // Create server name from package name
        let server_name = format!("{} MCP Server", package_name);
        
        // Create environment variables map from extracted env vars
        let mut env_map = HashMap::new();
        for var_name in env_vars {
            env_map.insert(
                var_name.clone(),
                ServerEnvironment {
                    description: format!("Extracted from README.md: {}", var_name),
                    default: Some("".to_string()), // Empty default value
                    required: true,
                },
            );
        }
        
        // Create configuration based on entry point
        let configuration = if let Some(script) = entry_point {
            Some(ServerConfiguration {
                command: Some("uv".to_string()),
                args: Some(vec!["run".to_string(), script]),
                env: Some(env_map),
            })
        } else {
            // Fallback to python -m if no script found
            Some(ServerConfiguration {
                command: Some("python".to_string()),
                args: Some(vec!["-m".to_string(), package_name.replace("-", "_")]),
                env: Some(env_map),
            })
        };
        
        // Create distribution
        let distribution = Some(Distribution {
            r#type: "python".to_string(),
            package: package_name,
        });
        
        // Create registration request
        let request = ServerRegistrationRequest {
            server_id,
            server_name,
            description,
            tools_type: "python".to_string(),
            configuration,
            distribution,
        };
        
        // Register the server
        self.register_server(request).await
    }
}
