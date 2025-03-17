use crate::mcp_protocol::{discover_server_tools, execute_server_tool};
use crate::mcp_state::mcp_state_process_utils::{kill_process, spawn_process};
use crate::models::types::{ServerStatus, ServerToolInfo};
use crate::registry::server_registry::ServerRegistry;
use crate::MCPError;
use async_trait::async_trait;
use log::{error, info, warn};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use super::process_manager::ProcessManager;

/// MCPState: the main service layer
///
/// This module coordinates database operations, process management, and discovered tools.
/// It serves as the central orchestration layer that connects the database (ServerRegistry),
/// process management (ProcessManager), and JSON-RPC operations (mcp_proxy).
#[derive(Clone)]
pub struct MCPState {
    pub tool_registry: Arc<RwLock<ServerRegistry>>,
    pub process_manager: Arc<RwLock<ProcessManager>>,
    pub server_tools: Arc<RwLock<HashMap<String, Vec<ServerToolInfo>>>>,
    pub are_tools_hidden: Arc<RwLock<bool>>,
}

impl MCPState {
    pub fn new(
        tool_registry: Arc<RwLock<ServerRegistry>>,
        process_manager: Arc<RwLock<ProcessManager>>,
        server_tools: Arc<RwLock<HashMap<String, Vec<ServerToolInfo>>>>,
    ) -> Self {
        // Initialize with default value
        let are_tools_hidden = Arc::new(RwLock::new(false));
        
        // Create the state instance
        let state = Self {
            tool_registry,
            process_manager,
            server_tools,
            are_tools_hidden,
        };
        
        // Return the state instance
        state
    }
    
    /// Initialize the state from the database
    pub async fn init_state(&self) -> Result<(), String> {
        // Load the tool visibility state from the database
        let registry = self.tool_registry.read().await;
        match registry.get_setting("tools_hidden") {
            Ok(value) => {
                let hidden = value == "true";
                let mut are_tools_hidden = self.are_tools_hidden.write().await;
                *are_tools_hidden = hidden;
                info!("Loaded tools visibility state from database: {}", hidden);
            }
            Err(_) => {
                // Setting doesn't exist yet, use the default value (false)
                info!("No tools visibility state found in database, using default (visible)");
            }
        }
        
        Ok(())
    }

    /// Kill all running processes
    pub async fn kill_all_processes(&self) {
        let mut process_manager = self.process_manager.write().await;
        for (tool_id, process_opt) in process_manager.processes.iter_mut() {
            if let (Some(process), _) = process_opt {
                if let Err(e) = process.kill().await {
                    error!("Failed to kill process for tool {}: {}", tool_id, e);
                } else {
                    info!("Killed process for tool {}", tool_id);
                }
            }
        }
    }

    /// Execute a tool on a server
    pub async fn execute_tool(
        &self,
        server_id: &str,
        tool_id: &str,
        parameters: Value,
    ) -> Result<Value, MCPError> {
        let mut process_manager = self.process_manager.write().await;
        let (stdin, stdout) = process_manager
            .process_ios
            .get_mut(server_id)
            .ok_or_else(|| MCPError::ServerNotFound(server_id.to_string()))?;

        execute_server_tool(server_id, tool_id, parameters, stdin, stdout).await
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

        // Check if the process is already running
        let process_running = {
            let process_manager = self.process_manager.read().await;
            process_manager
                .processes
                .get(server_id)
                .is_some_and(|(p, status)| matches!(status, ServerStatus::Running) && p.is_some())
        };

        if process_running {
            info!(
                "Server {} is already running, killing process before restart",
                server_id
            );

            // Get the process and kill it
            {
                let mut process_manager = self.process_manager.write().await;
                if let Some((Some(process), _)) = process_manager.processes.get_mut(server_id) {
                    if let Err(e) = kill_process(process).await {
                        error!("Failed to kill process for server {}: {}", server_id, e);
                        return Err(format!("Failed to kill process: {}", e));
                    }
                    info!("Successfully killed process for server {}", server_id);
                }

                // Remove the process from the registry and set status to Stopped
                process_manager
                    .processes
                    .insert(server_id.to_string(), (None, ServerStatus::Stopped));

                // Remove the process IOs
                process_manager.process_ios.remove(server_id);
            }
        }

        // Check if the tool is enabled
        if !server_data.enabled {
            info!("Server {} is disabled, not restarting", server_id);
            return Ok(());
        }

        info!(
            "Server {} is enabled and not running, starting process",
            server_id
        );

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
                    .filter_map(|(k, tool_env)| tool_env.default.clone().map(|v| (k.clone(), v)))
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

        // Spawn process based on tool type
        let spawn_result = spawn_process(
            &config_value,
            server_id,
            &server_data.tools_type,
            env_vars.as_ref(),
        )
        .await;

        match spawn_result {
            Ok((process, stdin, stdout)) => {
                info!("Successfully spawned process for server: {}", server_id);
                {
                    let mut process_manager = self.process_manager.write().await;
                    process_manager
                        .processes
                        .insert(server_id.to_string(), (Some(process), ServerStatus::Starting));
                    process_manager
                        .process_ios
                        .insert(server_id.to_string(), (stdin, stdout));
                }

                // Wait longer for the server to start
                {
                    info!("Waiting for server to start for server: {}", server_id);
                    let sleep_future = tokio::time::sleep(Duration::from_secs(3));
                    sleep_future.await;
                    info!(
                        "Finished waiting for server to start for server: {}",
                        server_id
                    );
                }

                // Try to discover tools from this server
                match self.discover_server_tools(server_id).await {
                    Ok(tools) => {
                        {
                            let mut server_tools = self.server_tools.write().await;
                            server_tools.insert(server_id.to_string(), tools.clone());
                        }
                        info!(
                            "Successfully discovered {} tools for {}",
                            tools.len(),
                            server_id
                        );
                        
                        // Update the process status to Running after successful tool discovery
                        {
                            let mut process_manager = self.process_manager.write().await;
                            if let Some((child_opt, _)) = process_manager.processes.remove(server_id) {
                                process_manager.processes.insert(
                                    server_id.to_string(), 
                                    (child_opt, ServerStatus::Running)
                                );
                                info!("Updated status to Running for server: {}", server_id);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to discover tools from server {}: {}", server_id, e);
                        // Continue even if discovery fails
                    }
                }

                Ok(())
            }
            Err(e) => {
                error!("Failed to spawn process for server {}: {}", server_id, e);
                Err(format!("Failed to spawn process: {}", e))
            }
        }
    }

    /// Discover tools from a server
    pub async fn discover_server_tools(
        &self,
        server_id: &str,
    ) -> Result<Vec<ServerToolInfo>, String> {
        let mut process_manager = self.process_manager.write().await;
        let (stdin, stdout) = match process_manager.process_ios.get_mut(server_id) {
            Some(io) => io,
            None => return Err(format!("Server {} not found or not running", server_id)),
        };

        let tools = discover_server_tools(server_id, stdin, stdout).await?;

        // Update the process status to Running after successful tool discovery
        if let Some((child_opt, _)) = process_manager.processes.remove(server_id) {
            process_manager.processes.insert(
                server_id.to_string(), 
                (child_opt, ServerStatus::Running)
            );
            info!("Updated status to Running for server: {}", server_id);
        }

        // Update the server_tools map with the discovered tools
        let mut server_tools = self.server_tools.write().await;
        server_tools.insert(server_id.to_string(), tools.clone());

        // Save the tools to the database
        let registry = self.tool_registry.read().await;
        for tool in &tools {
            if let Err(e) = registry.save_server_tool(tool) {
                error!("Failed to save server tool to database: {}", e);
            }
        }

        Ok(tools)
    }

    /// Get the current tool visibility state
    pub async fn are_tools_hidden(&self) -> bool {
        let are_tools_hidden = self.are_tools_hidden.read().await;
        *are_tools_hidden
    }

    /// Set the tool visibility state
    pub async fn set_tools_hidden(&self, hidden: bool) -> Result<(), String> {
        // Update the in-memory state
        let mut are_tools_hidden = self.are_tools_hidden.write().await;
        *are_tools_hidden = hidden;
        info!("Tools visibility set to: {}", if hidden { "hidden" } else { "visible" });
        
        // Persist the state to the database
        let registry = self.tool_registry.read().await;
        registry.save_setting("tools_hidden", if hidden { "true" } else { "false" })
    }
}

#[async_trait]
pub trait McpStateProcessMonitor {
    async fn start_process_monitor(self);
}

#[async_trait]
impl McpStateProcessMonitor for Arc<RwLock<MCPState>> {
    // Start a background task that periodically checks if processes are running
    async fn start_process_monitor(self) {
        info!("Starting process monitor task");

        let self_clone_read_guard = self.read().await;
        let tool_registry = self_clone_read_guard.tool_registry.clone();
        let process_manager = self_clone_read_guard.process_manager.clone();
        drop(self_clone_read_guard);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));

            loop {
                interval.tick().await;

                // Get all tools from database
                let tools_result = {
                    let registry = tool_registry.read().await;
                    registry.get_all_servers()
                };

                let tools = match tools_result {
                    Ok(tools) => tools,
                    Err(e) => {
                        error!("Failed to get tools from database: {}", e);
                        continue;
                    }
                };

                // Check all enabled tools
                for (id_str, tool) in tools {
                    if tool.enabled {
                        // Check if process is running
                        let process_running = {
                            let process_manager = process_manager.read().await;
                            process_manager
                                .processes
                                .get(&id_str)
                                .is_some_and(|(p, status)| matches!(status, ServerStatus::Running) && p.is_some())
                        };

                        if !process_running {
                            warn!("Process for tool {} is not running but should be. Will attempt restart.", id_str);
                            let self_clone_write_guard = self.write().await;
                            if let Err(e) = self_clone_write_guard.restart_server(&id_str).await {
                                error!("Failed to restart tool {}: {}", id_str, e);
                            } else {
                                info!("Successfully restarted tool: {}", id_str);
                            }
                        }
                    }
                }
            }
        });
    }
}
