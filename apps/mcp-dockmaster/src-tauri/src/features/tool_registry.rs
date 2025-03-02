use log::{error, info};
use serde_json::{json, Value};
use std::{collections::HashMap, sync::Arc};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt},
    process::{Child, Command},
    sync::RwLock,
    time::Duration,
};
use thiserror::Error;

use crate::features::database::{Database, DbResult};

/// Holds information about registered tools and their processes
#[derive(Default)]
pub struct ToolRegistry {
    pub tools: HashMap<String, Value>,
    pub processes: HashMap<String, Option<Child>>,
    pub server_tools: HashMap<String, Vec<Value>>,
    pub process_ios: HashMap<String, (tokio::process::ChildStdin, tokio::process::ChildStdout)>,
    pub database: Option<Arc<Database>>,
}

#[derive(Error, Debug)]
pub enum MCPError {
    #[error("Server {0} not found or not running")]
    ServerNotFound(String),

    #[error("Failed to serialize command: {0}")]
    SerializationError(String),

    #[error("Failed to write to process stdin: {0}")]
    StdinWriteError(String),

    #[error("Failed to flush stdin: {0}")]
    StdinFlushError(String),

    #[error("Failed to read from process stdout: {0}")]
    StdoutReadError(String),

    #[error("Timeout waiting for response from server {0}")]
    TimeoutError(String),

    #[error("Failed to parse response as JSON: {0}")]
    JsonParseError(String),

    #[error("Tool execution error: {0}")]
    ToolExecutionError(String),

    #[error("Server process closed connection")]
    ServerClosedConnection,

    #[error("No response from process")]
    NoResponse,

    #[error("Response contains no result field")]
    NoResultField,
}

impl ToolRegistry {
    /// Create a new ToolRegistry with database support
    pub fn new(database: Arc<Database>) -> Self {
        Self {
            tools: HashMap::new(),
            processes: HashMap::new(),
            server_tools: HashMap::new(),
            process_ios: HashMap::new(),
            database: Some(database),
        }
    }

    /// Load tools and server tools from the database
    pub fn load_from_database(&mut self) -> DbResult<()> {
        if let Some(db) = &self.database {
            // Load tools
            self.tools = db.load_tools()?;
            
            // Load server tools
            self.server_tools = db.load_server_tools()?;
            
            info!("Loaded {} tools and {} server tools from database", 
                  self.tools.len(), self.server_tools.len());
        }
        
        Ok(())
    }

    /// Save a tool to the database
    pub fn save_tool(&self, id: &str, data: &Value) -> DbResult<()> {
        if let Some(db) = &self.database {
            db.save_tool(id, data)?;
        }
        
        Ok(())
    }

    /// Save server tools to the database
    pub fn save_server_tools(&self, server_id: &str, tools: &[Value]) -> DbResult<()> {
        if let Some(db) = &self.database {
            db.save_server_tools(server_id, tools)?;
        }
        
        Ok(())
    }

    /// Delete a tool from the database
    pub fn delete_tool(&self, id: &str) -> DbResult<()> {
        if let Some(db) = &self.database {
            db.delete_tool(id)?;
        }
        
        Ok(())
    }

    /// Delete server tools from the database
    pub fn delete_server_tools(&self, server_id: &str) -> DbResult<()> {
        if let Some(db) = &self.database {
            db.delete_server_tools(server_id)?;
        }
        
        Ok(())
    }

    /// Kill all running processes
    pub async fn kill_all_processes(&mut self) {
        for (tool_id, process_opt) in self.processes.iter_mut() {
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
        &mut self,
        server_id: &str,
        tool_id: &str,
        parameters: Value,
    ) -> Result<Value, MCPError> {
        execute_server_tool(server_id, tool_id, parameters, self).await
    }
}

/// Execute a tool on an MCP server
pub async fn execute_server_tool(
    server_id: &str,
    tool_name: &str,
    parameters: Value,
    registry: &mut ToolRegistry,
) -> Result<Value, MCPError> {
    let (stdin, stdout) = registry
        .process_ios
        .get_mut(server_id)
        .ok_or_else(|| MCPError::ServerNotFound(server_id.to_string()))?;

    let execute_cmd = json!({
        "jsonrpc": "2.0",
        "id": format!("execute_{}_{}", server_id, tool_name),
        "method": "tools/call",
        "params": { "name": tool_name, "arguments": parameters }
    });

    let cmd_str = serde_json::to_string(&execute_cmd)
        .map_err(|e| MCPError::SerializationError(e.to_string()))?
        + "\n";

    stdin
        .write_all(cmd_str.as_bytes())
        .await
        .map_err(|e| MCPError::StdinWriteError(e.to_string()))?;
    stdin
        .flush()
        .await
        .map_err(|e| MCPError::StdinFlushError(e.to_string()))?;

    let mut reader = tokio::io::BufReader::new(&mut *stdout);
    let mut response_line = String::new();

    let read_result = tokio::time::timeout(Duration::from_secs(30), reader.read_line(&mut response_line)).await;

    match read_result {
        Ok(Ok(0)) => return Err(MCPError::ServerClosedConnection),
        Ok(Ok(_)) => {},
        Ok(Err(e)) => return Err(MCPError::StdoutReadError(e.to_string())),
        Err(_) => return Err(MCPError::TimeoutError(server_id.to_string())),
    }

    if response_line.is_empty() {
        return Err(MCPError::NoResponse);
    }

    let response: Value = serde_json::from_str(&response_line)
        .map_err(|e| MCPError::JsonParseError(e.to_string()))?;

    if let Some(error) = response.get("error") {
        return Err(MCPError::ToolExecutionError(error.to_string()));
    }

    response
        .get("result")
        .cloned()
        .ok_or(MCPError::NoResultField)
}

/// Discover tools available from an MCP server
pub async fn discover_server_tools(server_id: &str, registry: &mut ToolRegistry) -> Result<Vec<Value>, String> {
    // Get the stdin/stdout handles for the server
    let (stdin, stdout) = match registry.process_ios.get_mut(server_id) {
        Some(io) => io,
        None => return Err(format!("Server {} not found or not running", server_id)),
    };
    
    info!("Discovering tools from server {}", server_id);
    
    // According to MCP specification, the correct method is "tools/list"
    // https://github.com/modelcontextprotocol/specification/blob/main/docs/specification/2024-11-05/server/tools.md
    let discover_cmd = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });
    
    // Send the command to the process
    let cmd_str = serde_json::to_string(&discover_cmd)
        .map_err(|e| format!("Failed to serialize command: {}", e))? + "\n";
    
    info!("Command: {}", cmd_str.trim());
    
    // Write command to stdin
    stdin.write_all(cmd_str.as_bytes()).await
        .map_err(|e| format!("Failed to write to process stdin: {}", e))?;
    stdin.flush().await
        .map_err(|e| format!("Failed to flush stdin: {}", e))?;
    
    // Read the response with a timeout
    let mut reader = tokio::io::BufReader::new(&mut *stdout);
    let mut response_line = String::new();
    
    let read_result = tokio::time::timeout(Duration::from_secs(10), reader.read_line(&mut response_line)).await;
    
    match read_result {
        Ok(Ok(0)) => return Err("Server process closed connection".to_string()),
        Ok(Ok(_)) => info!("Received response from server {}: {}", server_id, response_line.trim()),
        Ok(Err(e)) => return Err(format!("Failed to read from process stdout: {}", e)),
        Err(_) => return Err(format!("Timeout waiting for response from server {}", server_id)),
    }
    
    if response_line.is_empty() {
        return Err("No response from process".to_string());
    }
    
    // Parse the response
    let response: Value = match serde_json::from_str(&response_line) {
        Ok(json) => json,
        Err(e) => return Err(format!("Failed to parse response as JSON: {}", e)),
    };
    
    // Check for error in the response
    if let Some(error) = response.get("error") {
        return Err(format!("Server returned error: {:?}", error));
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

/// Shared state for MCP tools
#[derive(Clone)]
pub struct MCPState {
    pub tool_registry: Arc<RwLock<ToolRegistry>>,
    pub database: Option<Arc<Database>>,
}

impl Default for MCPState {
    fn default() -> Self {
        Self {
            tool_registry: Arc::new(RwLock::new(ToolRegistry::default())),
            database: None,
        }
    }
}

impl MCPState {
    /// Create a new MCPState with database support
    pub fn new(database: Arc<Database>) -> Self {
        let tool_registry = Arc::new(RwLock::new(ToolRegistry::new(database.clone())));
        
        Self {
            tool_registry,
            database: Some(database),
        }
    }
} 