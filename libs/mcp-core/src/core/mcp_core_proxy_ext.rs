use crate::models::types::{
    DiscoverServerToolsRequest, Distribution, RuntimeServer, ServerConfigUpdateRequest,
    ServerConfiguration, ServerDefinition, ServerEnvironment, ServerId, ServerRegistrationRequest,
    ServerRegistrationResponse, ServerStatus, ServerUninstallResponse, ServerUpdateRequest,
    ToolConfigUpdateResponse, ToolExecutionRequest, ToolExecutionResponse, ToolUninstallRequest,
    ToolUpdateResponse,
};
use crate::types::ServerToolInfo;
use crate::utils::github::{
    extract_env_vars_from_readme, fetch_github_file, parse_github_url, GitHubRepo,
};
use anyhow::Result;
use async_trait::async_trait;
use futures::future;
use log::{error, info};
use reqwest::Client;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
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
    async fn import_server_from_url(
        &self,
        github_url: String,
    ) -> Result<ServerRegistrationResponse, String>;
    /// Process a Node.js project from package.json content
    async fn process_nodejs_project(
        &self,
        package_json_content: String,
        repo_info: &GitHubRepo,
        env_vars: HashSet<String>,
    ) -> Result<ServerRegistrationResponse, String>;
    /// Process a Python project from pyproject.toml content
    async fn process_python_project(
        &self,
        pyproject_toml_content: String,
        repo_info: &GitHubRepo,
        env_vars: HashSet<String>,
    ) -> Result<ServerRegistrationResponse, String>;
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
        } // All locks are released at this point

        // Now call restart_server after the locks have been released
        let mcp_state = mcp_state_clone.read().await;
        let _ = mcp_state.restart_server(&server_id).await;

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
            let status = {
                let mcp_clients = mcp_state.mcp_clients.read().await;
                if let Some(mcp_client) = mcp_clients.get(&id) {
                    mcp_client.server_status.clone()
                } else {
                    ServerStatus::Stopped
                }
            };

            let tool_count = {
                let server_tools = mcp_state.server_tools.read().await;
                server_tools.get(&id).map_or(0, |tools| tools.len())
            };

            tools.push(RuntimeServer {
                definition: tool_struct,
                id: ServerId::new(id),
                status,
                tool_count,
            });
        }

        Ok(tools)
    }

    /// List all available tools from all running MCP servers
    async fn list_all_server_tools(&self) -> Result<Vec<ServerToolInfo>, String> {
        let mcp_state = self.mcp_state.read().await;

        // Check if tools are hidden
        if mcp_state.are_tools_hidden().await {
            // Return empty list when tools are hidden
            return Ok(Vec::new());
        }

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
        mcp_state.discover_server_tools(&request.server_id).await
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
        let mcp_clients = mcp_state.mcp_clients.read().await;
        let mcp_client = mcp_clients.get(server_id);
        if mcp_client.is_none() {
            return Err(format!("Server with ID '{}' not found", server_id));
        }
        let mcp_client = mcp_client.unwrap();

        // Check if server is stopped
        // if matches!(mcp_client.server_status, ServerStatus::Stopped) {
        //     return Err(format!("Server '{}' is stopped", server_id));
        // }

        let result = match mcp_client
            .client
            .call_tool(rmcp::model::CallToolRequestParam {
                name: tool_id.to_string().into(),
                arguments: request.parameters.clone(),
            })
            .await
        {
            Ok(result) => result,
            Err(e) => return Err(format!("Tool execution error: {}", e)),
        };

        Ok(ToolExecutionResponse {
            success: true,
            result: Some(serde_json::to_value(result).unwrap()),
            error: None,
        })
    }

    /// Update a tool's status (enabled/disabled)
    async fn update_server_status(
        &self,
        request: ServerUpdateRequest,
    ) -> Result<ToolUpdateResponse, String> {
        info!(
            "Updating tool status for: {} to {}",
            request.server_id, request.enabled
        );

        // First, check if the tool exists and get the necessary information
        let tool_info = {
            let mcp_state = self.mcp_state.read().await;
            let registry = mcp_state.tool_registry.read().await;

            if let Ok(tool) = registry.get_server(&request.server_id) {
                // Extract and clone the necessary values
                let tools_type = tool.tools_type.clone();
                let entry_point = tool.entry_point.clone().unwrap_or_default();
                let status = {
                    let mcp_clients = mcp_state.mcp_clients.read().await;
                    if let Some(mcp_client) = mcp_clients.get(&request.server_id) {
                        mcp_client.server_status.clone()
                    } else {
                        ServerStatus::Stopped
                    }
                };

                Some((tools_type, entry_point, status))
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
        // First update the tool's enabled status in registry
        {
            let mcp_state = self.mcp_state.read().await;
            let registry = mcp_state.tool_registry.write().await;
            let mut tool = registry.get_server(&request.server_id)?;
            tool.enabled = request.enabled;
            registry.save_server(&request.server_id, &tool)?;
        } // Registry lock is dropped here

        let result = if request.enabled {
            // If enabling, first set status to Starting
            self.mcp_state
                .read()
                .await
                .restart_server(&request.server_id)
                .await
        } else {
            // If disabling, shut down the server
            self.mcp_state
                .read()
                .await
                .kill_process(&request.server_id)
                .await
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
                "Tool '{}' status updated to {} and process {}",
                request.server_id,
                if request.enabled {
                    "enabled"
                } else {
                    "disabled"
                },
                if request.enabled {
                    "started"
                } else {
                    "stopped"
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
        if let Err(e) = mcp_state.kill_process(&request.server_id).await {
            error!(
                "Failed to kill process for server {}: {}",
                request.server_id, e
            );
        }

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

    /// Initialize and start background mcp services
    async fn init_mcp_server(&self) -> Result<()> {
        info!("Starting background initialization of MCP services");

        // Initialize the state from the database
        if let Err(e) = self.mcp_state.read().await.init_state().await {
            error!("Failed to initialize state from database: {}", e);
        }

        // Get all tools from database
        let tools = match self.tool_registry.read().await.get_all_servers() {
            Ok(tools) => tools,
            Err(e) => {
                error!("Failed to get tools from database: {}", e);
                return Err(anyhow::anyhow!("Failed to get tools from database: {}", e));
            }
        };

        info!("MCP state initialized, preparing to restart enabled tools");

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
                    let mcp_state_clone_write_guard = mcp_state_arc_clone.read().await;
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

        Ok(())
    }

    /// Kill all running processes
    async fn kill_all_processes(&self) -> Result<()> {
        let mcp_state = self.mcp_state.read().await;
        match mcp_state.kill_all_processes().await {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow::anyhow!("Failed to kill all processes: {}", e)),
        }
    }

    /// Import a server from a GitHub repository URL
    async fn import_server_from_url(
        &self,
        github_url: String,
    ) -> Result<ServerRegistrationResponse, String> {
        info!("Importing server from URL: {}", github_url);

        // Parse GitHub URL
        let repo_info = parse_github_url(&github_url)?;
        info!(
            "Parsed GitHub URL: owner={}, repo={}",
            repo_info.owner, repo_info.repo
        );

        // Create HTTP client
        let client = Client::builder()
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        // Try to fetch README.md to extract environment variables
        let mut env_vars = HashSet::new();
        let readme_result =
            fetch_github_file(&client, &repo_info.owner, &repo_info.repo, "README.md").await;

        if let Ok(readme_content) = readme_result {
            info!("Found README.md, extracting environment variables");
            env_vars = extract_env_vars_from_readme(&readme_content);
            info!(
                "Extracted {} environment variables from README.md",
                env_vars.len()
            );

            if !env_vars.is_empty() {
                info!("Extracted environment variables: {:?}", env_vars);
            }
        } else {
            info!("README.md not found or could not be parsed");
        }

        // Try to fetch package.json first (Node.js project)
        let package_json_result =
            fetch_github_file(&client, &repo_info.owner, &repo_info.repo, "package.json").await;

        if let Ok(package_json_content) = package_json_result {
            info!("Found package.json, processing as Node.js project");
            return self
                .process_nodejs_project(package_json_content, &repo_info, env_vars)
                .await;
        }

        // If package.json not found, try pyproject.toml (Python project)
        let pyproject_toml_result =
            fetch_github_file(&client, &repo_info.owner, &repo_info.repo, "pyproject.toml").await;

        if let Ok(pyproject_toml_content) = pyproject_toml_result {
            info!("Found pyproject.toml, processing as Python project");
            return self
                .process_python_project(pyproject_toml_content, &repo_info, env_vars)
                .await;
        }

        // If neither found, return error
        Err(format!(
            "Could not find package.json or pyproject.toml in repository {}/{}",
            repo_info.owner, repo_info.repo
        ))
    }

    /// Process a Node.js project from package.json content
    async fn process_nodejs_project(
        &self,
        package_json_content: String,
        repo_info: &GitHubRepo,
        env_vars: HashSet<String>,
    ) -> Result<ServerRegistrationResponse, String> {
        // Parse package.json
        let package_json: Value = serde_json::from_str(&package_json_content)
            .map_err(|e| format!("Failed to parse package.json: {}", e))?;

        // Extract package name
        let package_name = package_json
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'name' field in package.json".to_string())?
            .to_string();

        // Extract description
        let description = package_json
            .get("description")
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
    async fn process_python_project(
        &self,
        pyproject_toml_content: String,
        repo_info: &GitHubRepo,
        env_vars: HashSet<String>,
    ) -> Result<ServerRegistrationResponse, String> {
        // Parse pyproject.toml
        let pyproject_toml: Table = pyproject_toml_content
            .parse::<Table>()
            .map_err(|e| format!("Failed to parse pyproject.toml: {}", e))?;

        // Extract package name
        let project = pyproject_toml
            .get("project")
            .ok_or_else(|| "Missing 'project' section in pyproject.toml".to_string())?
            .as_table()
            .ok_or_else(|| "Invalid 'project' section in pyproject.toml".to_string())?;

        let package_name = project
            .get("name")
            .ok_or_else(|| "Missing 'name' field in pyproject.toml".to_string())?
            .as_str()
            .ok_or_else(|| "Invalid 'name' field in pyproject.toml".to_string())?
            .to_string();

        // Extract description
        let description = project
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("MCP Server imported from GitHub")
            .to_string();

        // Check for project.scripts section to determine entry point
        let entry_point = if let Some(scripts) = project.get("scripts").and_then(|v| v.as_table()) {
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
                command: Some("uvx".to_string()),
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
