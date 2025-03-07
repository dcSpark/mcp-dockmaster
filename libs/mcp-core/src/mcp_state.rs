use log::{error, info, warn};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use crate::mcp_proxy::{discover_server_tools, execute_server_tool, kill_process, spawn_process};
use crate::process_manager::ProcessManager;
use crate::registry::ToolRegistry;
use crate::MCPError;

/// MCPState: the main service layer
///
/// This module coordinates database operations, process management, and discovered tools.
/// It serves as the central orchestration layer that connects the database (ToolRegistry),
/// process management (ProcessManager), and JSON-RPC operations (mcp_proxy).
#[derive(Clone)]
pub struct MCPState {
    pub tool_registry: Arc<RwLock<ToolRegistry>>,
    pub process_manager: Arc<RwLock<ProcessManager>>,
    pub server_tools: Arc<RwLock<HashMap<String, Vec<Value>>>>,
}

impl Default for MCPState {
    fn default() -> Self {
        Self::new()
    }
}

impl MCPState {
    pub fn new() -> Self {
        let registry = ToolRegistry::new()
            .expect("Failed to create ToolRegistry during MCPState initialization");
        Self {
            tool_registry: Arc::new(RwLock::new(registry)),
            process_manager: Arc::new(RwLock::new(ProcessManager::new())),
            server_tools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Kill all running processes
    pub async fn kill_all_processes(&self) {
        let mut process_manager = self.process_manager.write().await;
        for (tool_id, process_opt) in process_manager.processes.iter_mut() {
            if let Some(process) = process_opt {
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

    /// Restart a tool by its ID
    pub async fn restart_tool(&self, tool_id: &str) -> Result<(), String> {
        info!("Attempting to restart tool: {}", tool_id);

        // Get tool from database
        let tool_data = {
            let registry = self.tool_registry.read().await;
            registry.get_tool(tool_id)?
        };

        // Check if tool_type is empty
        if tool_data.tool_type.is_empty() {
            error!("Missing tool_type for tool {}", tool_id);
            return Err(format!("Missing tool_type for tool {}", tool_id));
        }

        // Check if the process is already running
        let process_running = {
            let process_manager = self.process_manager.read().await;
            process_manager
                .processes
                .get(tool_id)
                .is_some_and(|p| p.is_some())
        };

        if process_running {
            info!(
                "Tool {} is already running, killing process before restart",
                tool_id
            );

            // Get the process and kill it
            {
                let mut process_manager = self.process_manager.write().await;
                if let Some(Some(process)) = process_manager.processes.get_mut(tool_id) {
                    if let Err(e) = kill_process(process).await {
                        error!("Failed to kill process for tool {}: {}", tool_id, e);
                        return Err(format!("Failed to kill process: {}", e));
                    }
                    info!("Successfully killed process for tool {}", tool_id);
                }

                // Remove the process from the registry
                process_manager.processes.insert(tool_id.to_string(), None);

                // Remove the process IOs
                process_manager.process_ios.remove(tool_id);
            }
        }

        // Check if the tool is enabled
        if !tool_data.enabled {
            info!("Tool {} is disabled, not restarting", tool_id);
            return Ok(());
        }

        info!(
            "Tool {} is enabled and not running, starting process",
            tool_id
        );

        // Extract environment variables from the tool configuration
        let env_vars = if let Some(configuration) = &tool_data.configuration {
            if let Some(env_map) = &configuration.env {
                info!(
                    "Extracted {} environment variables for tool {}",
                    env_map.len(),
                    tool_id
                );
                Some(env_map.clone())
            } else {
                info!("No environment variables found for tool {}", tool_id);
                None
            }
        } else {
            info!("No configuration found for tool {}", tool_id);
            None
        };

        // Get the configuration from the tool data
        let config_value = if let Some(configuration) = &tool_data.configuration {
            info!("Using configuration from tool data for {}", tool_id);
            json!({
                "command": configuration.command,
                "args": configuration.args
            })
        } else if !tool_data.entry_point.clone().unwrap_or_default().is_empty() {
            info!(
                "Creating simple configuration with entry_point for {}",
                tool_id
            );
            json!({
                "command": tool_data.entry_point
            })
        } else {
            error!("Missing configuration for tool {}", tool_id);
            return Err(format!("Missing configuration for tool {}", tool_id));
        };

        // Spawn process based on tool type
        let spawn_result = spawn_process(
            &config_value,
            tool_id,
            &tool_data.tool_type,
            env_vars.as_ref(),
        )
        .await;

        match spawn_result {
            Ok((process, stdin, stdout)) => {
                info!("Successfully spawned process for tool: {}", tool_id);
                {
                    let mut process_manager = self.process_manager.write().await;
                    process_manager
                        .processes
                        .insert(tool_id.to_string(), Some(process));
                    process_manager
                        .process_ios
                        .insert(tool_id.to_string(), (stdin, stdout));
                }

                // Wait longer for the server to start
                {
                    info!("Waiting for server to start for tool: {}", tool_id);
                    let sleep_future = tokio::time::sleep(Duration::from_secs(5));
                    sleep_future.await;
                    info!("Finished waiting for server to start for tool: {}", tool_id);
                }

                // Try to discover tools from this server
                match self.discover_server_tools(tool_id).await {
                    Ok(tools) => {
                        {
                            let mut server_tools = self.server_tools.write().await;
                            server_tools.insert(tool_id.to_string(), tools.clone());
                        }
                        info!(
                            "Successfully discovered {} tools for {}",
                            tools.len(),
                            tool_id
                        );
                    }
                    Err(e) => {
                        error!("Failed to discover tools from server {}: {}", tool_id, e);
                        // Continue even if discovery fails
                    }
                }

                Ok(())
            }
            Err(e) => {
                error!("Failed to spawn process for tool {}: {}", tool_id, e);
                Err(format!("Failed to spawn process: {}", e))
            }
        }
    }

    /// Discover tools from a server
    pub async fn discover_server_tools(&self, server_id: &str) -> Result<Vec<Value>, String> {
        let mut process_manager = self.process_manager.write().await;
        let (stdin, stdout) = match process_manager.process_ios.get_mut(server_id) {
            Some(io) => io,
            None => return Err(format!("Server {} not found or not running", server_id)),
        };

        discover_server_tools(server_id, stdin, stdout).await
    }

    // Start a background task that periodically checks if processes are running
    pub fn start_process_monitor(self: Arc<Self>) {
        info!("Starting process monitor task");
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));

            loop {
                interval.tick().await;

                // Get all tools from database
                let tools_result = {
                    let registry = self.tool_registry.read().await;
                    registry.get_all_tools()
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
                            let process_manager = self.process_manager.read().await;
                            process_manager
                                .processes
                                .get(&id_str)
                                .is_some_and(|p| p.is_some())
                        };

                        if !process_running {
                            warn!("Process for tool {} is not running but should be. Will attempt restart.", id_str);
                            if let Err(e) = self.restart_tool(&id_str).await {
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
