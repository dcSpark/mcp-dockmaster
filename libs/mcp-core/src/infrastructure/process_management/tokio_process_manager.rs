use std::collections::HashMap;
use async_trait::async_trait;
use log::info;
use serde_json::{json, Value};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, ChildStdout},
    time::{Duration, timeout},
};
use std::sync::Arc;

use crate::models::models::{ToolConfiguration, ToolId, ToolType};
use crate::dm_process::DMProcess;

use crate::domain::traits::ProcessManager;
use crate::domain::errors::DomainError;
use super::process_store::ProcessStore;

pub struct TokioProcessManager {
    store: Arc<ProcessStore>,
}

impl TokioProcessManager {
    pub fn new(store: Arc<ProcessStore>) -> Self {
        Self { store }
    }
    
    /// Helper method to write a command to a process stdin
    async fn write_command(
        &self,
        stdin: &mut ChildStdin,
        command: &str,
    ) -> Result<(), DomainError> {
        stdin
            .write_all(format!("{}\n", command).as_bytes())
            .await
            .map_err(|e| DomainError::StdinWriteError(e.to_string()))?;
        
        stdin
            .flush()
            .await
            .map_err(|e| DomainError::StdinFlushError(e.to_string()))?;
        
        Ok(())
    }
    
    /// Helper method to read a response from a process stdout
    async fn read_response(
        &self,
        stdout: &mut BufReader<&mut ChildStdout>,
        server_id: &str,
    ) -> Result<String, DomainError> {
        let mut response = String::new();
        
        match timeout(Duration::from_secs(10), stdout.read_line(&mut response)).await {
            Ok(result) => {
                result.map_err(|e| DomainError::StdoutReadError(e.to_string()))?;
                
                if response.is_empty() {
                    return Err(DomainError::ServerClosedConnection);
                }
                
                Ok(response)
            },
            Err(_) => Err(DomainError::TimeoutError(server_id.to_string())),
        }
    }
}

#[async_trait]
impl ProcessManager for TokioProcessManager {
    /// Spawn a new process
    async fn spawn_process(
        &self,
        configuration: &Value,
        tool_id: &str,
        tool_type: &str,
        env_vars: Option<&HashMap<String, String>>,
    ) -> Result<(Child, ChildStdin, ChildStdout), DomainError> {
        info!("Spawning process for tool: {}", tool_id);
        
        let command = configuration
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| DomainError::InvalidToolConfiguration("Missing command".to_string()))?;

        let args = configuration
            .get("args")
            .and_then(|v| v.as_array())
            .map(|args| {
                args.iter()
                    .filter_map(|arg| arg.as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let config = ToolConfiguration {
            command: command.to_string(),
            args: Some(args),
        };

        let tool_type = match tool_type {
            "node" => ToolType::Node,
            "python" => ToolType::Python,
            "docker" => ToolType::Docker,
            _ => return Err(DomainError::InvalidToolConfiguration(format!("Unsupported tool type: {}", tool_type))),
        };

        let tool_id_obj = ToolId::new(tool_id.to_string());
        
        // Use DMProcess to spawn the process
        let dm_process = DMProcess::new(&tool_id_obj, &tool_type, &config, env_vars)
            .await
            .map_err(|e| DomainError::ProcessError(e))?;
        
        // We need to create a new DMProcess to avoid ownership issues
        // since we can't clone Child, ChildStdin, and ChildStdout
        let result = DMProcess::new(&tool_id_obj, &tool_type, &config, env_vars)
            .await
            .map_err(|e| DomainError::ProcessError(e))?;
            
        // Store the original process in the ProcessStore
        self.store
            .insert_process(tool_id.to_string(), dm_process.child, (dm_process.stdin, dm_process.stdout))
            .await;
            
        // Return the new DMProcess values
        Ok((result.child, result.stdin, result.stdout))
    }
    
    /// Kill a running process
    async fn kill_process(&self, process: &mut Child) -> Result<(), DomainError> {
        process
            .kill()
            .await
            .map_err(|e| DomainError::ProcessError(format!("Failed to kill process: {}", e)))
    }
    
    /// Discover tools from a server
    async fn discover_server_tools(
        &self,
        server_id: &str,
        process_ios: &mut HashMap<String, (ChildStdin, ChildStdout)>,
    ) -> Result<Vec<Value>, DomainError> {
        // Get the process I/O for the server
        let io_pair = process_ios
            .get_mut(server_id)
            .ok_or_else(|| DomainError::ServerNotFound(server_id.to_string()))?;
        
        let (stdin, stdout) = &mut *io_pair;
        
        info!("Discovering tools from server {}", server_id);

        // According to MCP specification, the correct method is "tools/list"
        let discover_cmd = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list",
            "params": {}
        });

        // Send the command to the process
        let cmd_str = serde_json::to_string(&discover_cmd)
            .map_err(|e| DomainError::SerializationError(format!("Failed to serialize command: {}", e)))?
            + "\n";

        info!("Command: {}", cmd_str.trim());

        // Write command to stdin
        stdin
            .write_all(cmd_str.as_bytes())
            .await
            .map_err(|e| DomainError::StdinWriteError(format!("Failed to write to process stdin: {}", e)))?;
        stdin
            .flush()
            .await
            .map_err(|e| DomainError::StdinFlushError(format!("Failed to flush stdin: {}", e)))?;

        // Read the response with a timeout
        let mut reader = tokio::io::BufReader::new(&mut *stdout);
        let mut response_line = String::new();

        let read_result = tokio::time::timeout(
            Duration::from_secs(10),
            reader.read_line(&mut response_line),
        )
        .await;

        match read_result {
            Ok(Ok(0)) => return Err(DomainError::ServerClosedConnection),
            Ok(Ok(_)) => info!(
                "Received response from server {}: {}",
                server_id,
                response_line.trim()
            ),
            Ok(Err(e)) => return Err(DomainError::StdoutReadError(format!("Failed to read from process stdout: {}", e))),
            Err(_) => {
                return Err(DomainError::TimeoutError(format!(
                    "Timeout waiting for response from server {}",
                    server_id
                )))
            }
        }

        if response_line.is_empty() {
            return Err(DomainError::NoResponse);
        }

        // Parse the response
        let response: Value = serde_json::from_str(&response_line)
            .map_err(|e| DomainError::JsonParseError(format!("Failed to parse response as JSON: {}", e)))?;

        // Check for error in the response
        if let Some(error) = response.get("error") {
            return Err(DomainError::ToolExecutionError(format!("Server returned error: {:?}", error)));
        }

        // According to MCP spec, tools should be in the result field
        if let Some(result) = response.get("result") {
            // MCP returns tools directly in the result field as array
            if let Some(tools_array) = result.as_array() {
                info!("Found {} tools in result array", tools_array.len());
                return Ok(tools_array.clone());
            }

            // Some implementations might nest it under a tools field
            if let Some(tools) = result.get("tools") {
                if let Some(tools_array) = tools.as_array() {
                    info!("Found {} tools in result.tools array", tools_array.len());
                    return Ok(tools_array.clone());
                }
            }

            // If there's a result but we couldn't find tools array, try to use the entire result
            info!("No tools array found, using entire result as fallback");
            return Ok(vec![result.clone()]);
        }

        // If the server doesn't fully comply with MCP but has a tools field at root
        if let Some(tools) = response.get("tools") {
            if let Some(tools_array) = tools.as_array() {
                info!("Found {} tools in root tools array", tools_array.len());
                return Ok(tools_array.clone());
            }
        }

        // If initialization hasn't completed yet or tools are not supported,
        // return an empty array as fallback
        info!("No tools found in response: {}", response_line.trim());
        Ok(Vec::new())
    }
    
    /// Execute a tool on a server
    async fn execute_server_tool(
        &self,
        server_id: &str,
        tool_id: &str,
        parameters: Value,
        process_ios: &mut HashMap<String, (ChildStdin, ChildStdout)>,
    ) -> Result<Value, DomainError> {
        // Get the process I/O for the server
        let io_pair = process_ios
            .get_mut(server_id)
            .ok_or_else(|| DomainError::ServerNotFound(server_id.to_string()))?;
        
        let (stdin, stdout) = &mut *io_pair;
        
        // Create the execute command according to MCP specification
        let execute_cmd = json!({
            "jsonrpc": "2.0",
            "id": format!("execute_{}_{}", server_id, tool_id),
            "method": "tools/call",
            "params": { "name": tool_id, "arguments": parameters }
        });
        
        // Serialize the command to a string
        let cmd_str = serde_json::to_string(&execute_cmd)
            .map_err(|e| DomainError::SerializationError(e.to_string()))?
            + "\n";
        
        // Write the command to stdin
        stdin
            .write_all(cmd_str.as_bytes())
            .await
            .map_err(|e| DomainError::StdinWriteError(e.to_string()))?;
        stdin
            .flush()
            .await
            .map_err(|e| DomainError::StdinFlushError(e.to_string()))?;
        
        // Read the response with a timeout
        let mut reader = tokio::io::BufReader::new(&mut *stdout);
        let mut response_line = String::new();
        
        let read_result = tokio::time::timeout(
            Duration::from_secs(30),
            reader.read_line(&mut response_line),
        )
        .await;
        
        match read_result {
            Ok(Ok(0)) => return Err(DomainError::ServerClosedConnection),
            Ok(Ok(_)) => {},
            Ok(Err(e)) => return Err(DomainError::StdoutReadError(e.to_string())),
            Err(_) => return Err(DomainError::TimeoutError(server_id.to_string())),
        }
        
        if response_line.is_empty() {
            return Err(DomainError::NoResponse);
        }
        
        // Parse the response as JSON
        let response: Value = serde_json::from_str(&response_line)
            .map_err(|e| DomainError::JsonParseError(e.to_string()))?;
        
        // Check for error in the response
        if let Some(error) = response.get("error") {
            return Err(DomainError::ToolExecutionError(error.to_string()));
        }
        
        // Extract the result
        response
            .get("result")
            .cloned()
            .ok_or(DomainError::NoResultField)
    }
    
    /// Kill all processes managed by this process manager
    async fn kill_all_processes(&self) -> Result<(), DomainError> {
        // Use the ProcessStore to kill all processes
        self.store.kill_all_processes().await;
        Ok(())
    }
}
