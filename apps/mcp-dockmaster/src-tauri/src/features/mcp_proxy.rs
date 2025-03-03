use log::{error, info};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::HashMap, sync::Arc};
use std::process::Stdio;
use tauri::State;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt},
    process::{Child, Command},
    sync::RwLock,
    time::Duration,
};
use thiserror::Error;

/// Holds information about registered tools and their processes
#[derive(Default)]
pub struct ToolRegistry {
    pub tools: HashMap<String, Value>,
    pub processes: HashMap<String, Option<Child>>,
    pub server_tools: HashMap<String, Vec<Value>>,
    pub process_ios: HashMap<String, (tokio::process::ChildStdin, tokio::process::ChildStdout)>,
}

impl ToolRegistry {
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

/// Shared state for MCP tools
#[derive(Clone, Default)]
pub struct MCPState {
    pub tool_registry: Arc<RwLock<ToolRegistry>>,
}

/// MCP tool registration request
#[derive(Deserialize)]
pub struct ToolRegistrationRequest {
    tool_name: String,
    description: String,
    authentication: Option<Value>,
    tool_type: String,  // "node", "python", "docker"
    configuration: Option<Value>,
    distribution: Option<Value>,
}

/// MCP tool registration response
#[derive(Serialize)]
pub struct ToolRegistrationResponse {
    success: bool,
    message: String,
    tool_id: Option<String>,
}

/// MCP tool execution request
#[derive(Deserialize)]
pub struct ToolExecutionRequest {
    tool_id: String,
    parameters: Value,
}

/// MCP tool execution response
#[derive(Serialize)]
pub struct ToolExecutionResponse {
    success: bool,
    result: Option<Value>,
    error: Option<String>,
}

/// MCP tool update request
#[derive(Deserialize)]
pub struct ToolUpdateRequest {
    tool_id: String,
    enabled: bool,
}

/// MCP tool update response
#[derive(Serialize)]
pub struct ToolUpdateResponse {
    success: bool,
    message: String,
}

/// MCP tool config update request
#[derive(Deserialize)]
pub struct ToolConfigUpdateRequest {
    tool_id: String,
    config: ToolConfig,
}

/// MCP tool config
#[derive(Deserialize)]
pub struct ToolConfig {
    env: HashMap<String, String>,
}

/// MCP tool config update response
#[derive(Serialize)]
pub struct ToolConfigUpdateResponse {
    success: bool,
    message: String,
}

/// MCP tool uninstall request
#[derive(Deserialize)]
pub struct ToolUninstallRequest {
    tool_id: String,
}

/// MCP tool uninstall response
#[derive(Serialize)]
pub struct ToolUninstallResponse {
    success: bool,
    message: String,
}

/// MCP server discovery request
#[derive(Deserialize)]
pub struct DiscoverServerToolsRequest {
    server_id: String,
}

/// MCP server discovery response
#[derive(Serialize)]
pub struct DiscoverServerToolsResponse {
    success: bool,
    tools: Option<Vec<Value>>,
    error: Option<String>,
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

    // #[error("Entry point file '{0}' does not exist")]
    // EntryPointNotFound(String),

    // #[error("Failed to spawn process: {0}")]
    // ProcessSpawnError(String),

    // #[error("Failed to open stdin")]
    // StdinOpenError,

    // #[error("Failed to open stdout")]
    // StdoutOpenError,

    #[error("Server process closed connection")]
    ServerClosedConnection,

    #[error("No response from process")]
    NoResponse,

    #[error("Response contains no result field")]
    NoResultField,
}

/// Discover tools available from an MCP server
async fn discover_server_tools(server_id: &str, registry: &mut ToolRegistry) -> Result<Vec<Value>, String> {
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

/// Execute a tool on an MCP server
async fn execute_server_tool(
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

/// Spawn a Node.js MCP server process
async fn spawn_nodejs_process(configuration: &Value, tool_id: &str, env_vars: Option<&HashMap<String, String>>) -> Result<(Child, tokio::process::ChildStdin, tokio::process::ChildStdout), String> {  
    let mut cmd;

    let command = configuration.get("command").and_then(|v| v.as_str()).ok_or_else(|| 
        format!("Configuration missing 'command' field or not a string"))?;

    if command.contains("npx") || command.contains("node") {
        info!("Using npx to run npm package: {}", command);
        cmd = Command::new(command);        
        // Add args if they exist
        info!("Args: {:?}", configuration.get("args"));
        if let Some(args) = configuration.get("args").and_then(|v| v.as_array()) {
            for arg in args {
                if let Some(arg_str) = arg.as_str() {
                    cmd.arg(arg_str);
                }
            }
        }

        cmd.arg("--tool-id")
        .arg(tool_id);
        
        cmd.stdin(Stdio::piped())
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());        
    } else {
        // Otherwise, assume it's a file path that doesn't exist yet
        info!("Entry point doesn't exist and doesn't look like an npm package or node command: {}", command);
        return Err(format!("Entry point file '{}' does not exist", command));
    }
    
    // Add environment variables if provided
    if let Some(env_map) = env_vars {
        for (key, value) in env_map {
            info!("Setting environment variable: {}={}", key, value);
            cmd.env(key, value);
        }
    }
    
    // Log the command we're about to run
    info!("Spawning process: {:?} with args: {:?}", cmd.as_std().get_program(), cmd.as_std().get_args().collect::<Vec<_>>());
    
    // Spawn the process
    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(e) => {
            error!("Failed to spawn process: {}", e);
            return Err(format!("Failed to spawn process: {}", e));
        }
    };
    
    // Capture stderr to a separate task that logs errors
    if let Some(stderr) = child.stderr.take() {
        let tool_id_clone = tool_id.to_string();
        tokio::spawn(async move {
            let mut stderr_reader = tokio::io::BufReader::new(stderr);
            let mut line = String::new();
            while let Ok(bytes_read) = stderr_reader.read_line(&mut line).await {
                if bytes_read == 0 {
                    break;
                }
                info!("[{} stderr]: {}", tool_id_clone, line.trim());
                line.clear();
            }
        });
    }
    
    // Take ownership of the pipes
    let stdin = match child.stdin.take() {
        Some(stdin) => stdin,
        None => {
            error!("Failed to open stdin for process");
            if let Err(e) = child.kill().await {
                error!("Failed to kill process after stdin error: {}", e);
            }
            return Err(String::from("Failed to open stdin"));
        }
    };
    
    let stdout = match child.stdout.take() {
        Some(stdout) => stdout,
        None => {
            error!("Failed to open stdout for process");
            if let Err(e) = child.kill().await {
                error!("Failed to kill process after stdout error: {}", e);
            }
            return Err(String::from("Failed to open stdout"));
        }
    };
    
    info!("Process spawned successfully with stdin and stdout pipes");
    // Return the process and pipes
    Ok((child, stdin, stdout))
}

/// Spawn a Python MCP server process
async fn spawn_python_process(configuration: &Value, tool_id: &str, env_vars: Option<&HashMap<String, String>>) -> Result<(Child, tokio::process::ChildStdin, tokio::process::ChildStdout), String> {  
    let mut cmd;

    let command = configuration.get("command").and_then(|v| v.as_str()).ok_or_else(|| 
        format!("Configuration missing 'command' field or not a string"))?;

    info!("Using Python command: {}", command);
    cmd = Command::new(command);        
    
    // Add args if they exist
    info!("Args: {:?}", configuration.get("args"));
    if let Some(args) = configuration.get("args").and_then(|v| v.as_array()) {
        for arg in args {
            if let Some(arg_str) = arg.as_str() {
                cmd.arg(arg_str);
            }
        }
    }
    
    cmd.stdin(Stdio::piped())
       .stdout(Stdio::piped())
       .stderr(Stdio::piped());        
    
    // Add environment variables if provided
    if let Some(env_map) = env_vars {
        for (key, value) in env_map {
            info!("Setting environment variable: {}={}", key, value);
            cmd.env(key, value);
        }
    }
    
    // Log the command we're about to run
    info!("Spawning Python process: {:?} with args: {:?}", cmd.as_std().get_program(), cmd.as_std().get_args().collect::<Vec<_>>());
    
    // Spawn the process
    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(e) => {
            error!("Failed to spawn Python process: {}", e);
            return Err(format!("Failed to spawn Python process: {}", e));
        }
    };
    
    // Capture stderr to a separate task that logs errors
    if let Some(stderr) = child.stderr.take() {
        let tool_id_clone = tool_id.to_string();
        tokio::spawn(async move {
            let mut stderr_reader = tokio::io::BufReader::new(stderr);
            let mut line = String::new();
            while let Ok(bytes_read) = stderr_reader.read_line(&mut line).await {
                if bytes_read == 0 {
                    break;
                }
                info!("[{} stderr]: {}", tool_id_clone, line.trim());
                line.clear();
            }
        });
    }
    
    // Take ownership of the pipes
    let stdin = match child.stdin.take() {
        Some(stdin) => stdin,
        None => {
            error!("Failed to open stdin for Python process");
            if let Err(e) = child.kill().await {
                error!("Failed to kill process after stdin error: {}", e);
            }
            return Err(String::from("Failed to open stdin"));
        }
    };
    
    let stdout = match child.stdout.take() {
        Some(stdout) => stdout,
        None => {
            error!("Failed to open stdout for Python process");
            if let Err(e) = child.kill().await {
                error!("Failed to kill process after stdout error: {}", e);
            }
            return Err(String::from("Failed to open stdout"));
        }
    };
    
    info!("Python process spawned successfully with stdin and stdout pipes");
    // Return the process and pipes
    Ok((child, stdin, stdout))
}

/// Spawn a Docker MCP server process
async fn spawn_docker_process(configuration: &Value, tool_id: &str, env_vars: Option<&HashMap<String, String>>) -> Result<(Child, tokio::process::ChildStdin, tokio::process::ChildStdout), String> {  
    let command = configuration.get("command").and_then(|v| v.as_str()).ok_or_else(|| 
        format!("Configuration missing 'command' field or not a string"))?;

    if command != "docker" {
        return Err(format!("Expected 'docker' command for Docker runtime, got '{}'", command));
    }

    info!("Using Docker command");
    let mut cmd = Command::new(command);

    cmd.arg("run")
       .arg("-i")
       .arg("--name")
       .arg(tool_id)
       .arg("-a")
       .arg("stdout")
       .arg("-a")
       .arg("stderr")
       .arg("-a")
       .arg("stdin");
    
    // Add environment variables as Docker -e flags if provided
    if let Some(env_map) = env_vars {
        for (key, value) in env_map {
            info!("Setting Docker environment variable: {}={}", key, value);
            cmd.arg("-e")
               .arg(format!("{}={}", key, value));
        }
    }

    cmd.arg("--rm");

    // Add args if they exist
    info!("Args: {:?}", configuration.get("args"));
    if let Some(args) = configuration.get("args").and_then(|v| v.as_array()) {
        for arg in args {
            if let Some(arg_str) = arg.as_str() {
                cmd.arg(arg_str);
            }
        }
    }
    
    cmd.stdin(Stdio::piped())
       .stdout(Stdio::piped())
       .stderr(Stdio::piped());        
    
    // Log the command we're about to run
    info!("Spawning Docker process: {:?} with args: {:?}", cmd.as_std().get_program(), cmd.as_std().get_args().collect::<Vec<_>>());
    
    // Spawn the process
    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(e) => {
            error!("Failed to spawn Docker process: {}", e);
            return Err(format!("Failed to spawn Docker process: {}", e));
        }
    };
    
    // Capture stderr to a separate task that logs errors
    if let Some(stderr) = child.stderr.take() {
        let tool_id_clone = tool_id.to_string();
        tokio::spawn(async move {
            let mut stderr_reader = tokio::io::BufReader::new(stderr);
            let mut line = String::new();
            while let Ok(bytes_read) = stderr_reader.read_line(&mut line).await {
                if bytes_read == 0 {
                    break;
                }
                info!("[{} stderr]: {}", tool_id_clone, line.trim());
                line.clear();
            }
        });
    }
    
    // Take ownership of the pipes
    let stdin = match child.stdin.take() {
        Some(stdin) => stdin,
        None => {
            error!("Failed to open stdin for Docker process");
            if let Err(e) = child.kill().await {
                error!("Failed to kill process after stdin error: {}", e);
            }
            return Err(String::from("Failed to open stdin"));
        }
    };
    
    let stdout = match child.stdout.take() {
        Some(stdout) => stdout,
        None => {
            error!("Failed to open stdout for Docker process");
            if let Err(e) = child.kill().await {
                error!("Failed to kill process after stdout error: {}", e);
            }
            return Err(String::from("Failed to open stdout"));
        }
    };
    
    info!("Docker process spawned successfully with stdin and stdout pipes");
    // Return the process and pipes
    Ok((child, stdin, stdout))
}

/// Kill a running process
async fn kill_process(process: &mut Child) -> Result<(), String> {
    match process.kill().await {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Failed to kill process: {}", e)),
    }
}

/// Register a new tool with the MCP server
#[tauri::command]
pub async fn register_tool(
    state: State<'_, MCPState>,
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
    
    let mut registry = state.tool_registry.write().await;
    
    // Generate a simple tool ID (in production, use UUIDs)
    let tool_id = format!("tool_{}", registry.tools.len() + 1);
    info!("Generated tool ID: {}", tool_id);
    
    // Store the tool definition
    let mut tool_definition = json!({
        "name": request.tool_name,
        "description": request.description,
        "enabled": true, // Default to enabled
        "tool_type": request.tool_type,
        "configuration": request.configuration,
        "distribution": request.distribution,
    });
    
    // Add authentication if provided
    if let Some(auth) = &request.authentication {
        // Check if authentication contains environment variables
        if let Some(env) = auth.get("env") {
            // Store as config.env
            if let Some(obj) = tool_definition.as_object_mut() {
                obj.insert("config".to_string(), json!({
                    "env": env
                }));
            }
        } else {
            // Store as authentication
            if let Some(obj) = tool_definition.as_object_mut() {
                obj.insert("authentication".to_string(), auth.clone());
            }
        }
    }
    
    registry.tools.insert(tool_id.clone(), tool_definition);

    // Create a default empty tools list
    registry.server_tools.insert(tool_id.clone(), Vec::new());
    
    // Extract environment variables if they exist
    let env_vars = if let Some(auth) = &request.authentication {
        if let Some(env) = auth.get("env") {
            // Convert the JSON env vars to a HashMap<String, String>
            let mut env_map = HashMap::new();
            if let Some(env_obj) = env.as_object() {
                for (key, value) in env_obj {
                    // Extract the value as a string
                    let value_str = match value {
                        Value::String(s) => s.clone(),
                        Value::Number(n) => n.to_string(),
                        Value::Bool(b) => b.to_string(),
                        _ => {
                            // For objects, check if it has a description field (which means it's a template)
                            if let Value::Object(obj) = value {
                                if obj.contains_key("description") {
                                    // This is a template, so we don't have a value yet
                                    continue;
                                }
                            }
                            // For other types, convert to JSON string
                            value.to_string()
                        }
                    };
                    env_map.insert(key.clone(), value_str);
                }
            }
            Some(env_map)
        } else {
            None
        }
    } else {
        None
    };
    
    // Ensure configuration exists before passing to spawn process
    let config = match &request.configuration {
        Some(config) => config,
        None => return Err("Configuration is required for tools".to_string()),
    };
    
    // Create a JSON Value with the command field from the entry_point string
    let config_value = json!({
        "command": config.get("command").and_then(|v| v.as_str()).unwrap_or(""),
        "args": config.get("args").and_then(|v| v.as_array()).unwrap_or(&Vec::new())
    });
    
    // Spawn process based on tool type
    let spawn_result = match request.tool_type.as_str() {
        "node" => {
            info!("Spawning Node.js process for tool: {}", request.tool_name);
            spawn_nodejs_process(&config_value, &tool_id, env_vars.as_ref()).await
        },
        "python" => {
            info!("Spawning Python process for tool: {}", request.tool_name);
            spawn_python_process(&config_value, &tool_id, env_vars.as_ref()).await
        },
        "docker" => {
            info!("Spawning Docker process for tool: {}", request.tool_name);
            spawn_docker_process(&config_value, &tool_id, env_vars.as_ref()).await
        },
        _ => {
            info!("Unsupported tool type: {}", request.tool_type);
            return Err(format!("Unsupported tool type: {}", request.tool_type));
        }
    };
    
    match spawn_result {
        Ok((process, stdin, stdout)) => {
            info!("Process spawned successfully for tool ID: {}", tool_id);
            registry.processes.insert(tool_id.clone(), Some(process));
            registry.process_ios.insert(tool_id.clone(), (stdin, stdout));
            
            // Wait a moment for the server to start
            info!("Waiting for server to initialize...");
            drop(registry); // Release the lock during sleep
            tokio::time::sleep(Duration::from_secs(3)).await;
            
            // Try to discover tools from this server with a timeout to avoid hanging
            info!("Attempting to discover tools from server {}", tool_id);
            let discover_result = tokio::time::timeout(
                Duration::from_secs(15),
                async {
                    let mut registry = state.tool_registry.write().await;
                    discover_server_tools(&tool_id, &mut registry).await
                }
            ).await;
            
            // Handle the result of the discovery attempt
            match discover_result {
                Ok(Ok(tools)) => {
                    info!("Successfully discovered {} tools from {}", tools.len(), tool_id);
                    let mut registry = state.tool_registry.write().await;
                    // Clone tools before inserting to avoid the "moved value" error
                    let tools_clone = tools.clone();
                    registry.server_tools.insert(tool_id.clone(), tools);
                    
                    // If empty tools, add a default "main" tool
                    if tools_clone.is_empty() {
                        info!("No tools discovered, adding a default main tool");
                        let default_tool = json!({
                            "id": "main",
                            "name": request.tool_name,
                            "description": request.description
                        });
                        registry.server_tools.insert(tool_id.clone(), vec![default_tool]);
                    }
                },
                Ok(Err(e)) => {
                    error!("Error discovering tools from server {}: {}", tool_id, e);
                    // Add a default tool since discovery failed
                    let mut registry = state.tool_registry.write().await;
                    let default_tool = json!({
                        "id": "main",
                        "name": request.tool_name,
                        "description": request.description
                    });
                    registry.server_tools.insert(tool_id.clone(), vec![default_tool]);
                    info!("Added default tool for server {}", tool_id);
                },
                Err(_) => {
                    error!("Timeout while discovering tools from server {}", tool_id);
                    // Add a default tool since discovery timed out
                    let mut registry = state.tool_registry.write().await;
                    let default_tool = json!({
                        "id": "main",
                        "name": request.tool_name,
                        "description": request.description
                    });
                    registry.server_tools.insert(tool_id.clone(), vec![default_tool]);
                    info!("Added default tool for server {} after timeout", tool_id);
                }
            }
        },
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
#[tauri::command]
pub async fn list_tools(state: State<'_, MCPState>) -> Result<Vec<Value>, String> {
    let registry = state.tool_registry.read().await;
    let mut tools = Vec::new();
    
    for (id, tool_value) in registry.tools.iter() {
        // Clone the value so we can modify it
        let mut tool = tool_value.clone();
        
        // Ensure the tool has an ID field
        if let Some(obj) = tool.as_object_mut() {
            obj.insert("id".to_string(), json!(id));
            
            // Add process status
            let process_running = registry.processes.get(id).is_some_and(|p| p.is_some());
            obj.insert("process_running".to_string(), json!(process_running));
            
            // Add number of available tools from this server
            let server_tool_count = registry.server_tools.get(id).map_or_else(|| 0, |tools| tools.len());
            obj.insert("tool_count".to_string(), json!(server_tool_count));
        }
        
        tools.push(tool);
    }
    Ok(tools)
}

/// List all available tools from all running MCP servers
#[tauri::command]
pub async fn list_all_server_tools(state: State<'_, MCPState>) -> Result<Vec<Value>, String> {
    let registry = state.tool_registry.read().await;
    let mut all_tools = Vec::new();
    
    for (server_id, tools) in &registry.server_tools {
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
#[tauri::command]
pub async fn discover_tools(
    state: State<'_, MCPState>,
    request: DiscoverServerToolsRequest,
) -> Result<DiscoverServerToolsResponse, String> {
    // Check if the server exists and is running
    let server_running = {
        let registry = state.tool_registry.read().await;
        registry.processes.get(&request.server_id).is_some_and(|p| p.is_some())
    };
    
    if !server_running {
        return Ok(DiscoverServerToolsResponse {
            success: false,
            tools: None,
            error: Some(format!("Server with ID '{}' is not running", request.server_id)),
        });
    }
    
    // Discover tools from the server
    let mut registry = state.tool_registry.write().await;
    match discover_server_tools(&request.server_id, &mut registry).await {
        Ok(tools) => {
            // Store the discovered tools
            registry.server_tools.insert(request.server_id.clone(), tools.clone());
            
            Ok(DiscoverServerToolsResponse {
                success: true,
                tools: Some(tools),
                error: None,
            })
        },
        Err(e) => {
            Ok(DiscoverServerToolsResponse {
                success: false,
                tools: None,
                error: Some(format!("Failed to discover tools: {}", e)),
            })
        }
    }
}

/// Execute a tool from an MCP server
#[tauri::command]
pub async fn execute_proxy_tool(
    state: State<'_, MCPState>,
    request: ToolExecutionRequest,
) -> Result<ToolExecutionResponse, String> {
    // Extract server_id and tool_id from the proxy_id
    let parts: Vec<&str> = request.tool_id.split(':').collect();
    if parts.len() != 2 {
        return Err("Invalid tool_id format. Expected 'server_id:tool_id'".to_string());
    }
    
    let server_id = parts[0];
    let tool_id = parts[1];
    
    // Execute the tool on the server
    let mut registry = state.tool_registry.write().await;
    match execute_server_tool(server_id, tool_id, request.parameters.clone(), &mut registry).await {
        Ok(result) => {
            Ok(ToolExecutionResponse {
                success: true,
                result: Some(result),
                error: None,
            })
        },
        Err(e) => {
            Ok(ToolExecutionResponse {
                success: false,
                result: None,
                error: Some(e.to_string()),
            })
        }
    }
}

/// Update a tool's status (enabled/disabled)
#[tauri::command]
pub async fn update_tool_status(
    state: State<'_, MCPState>,
    request: ToolUpdateRequest,
) -> Result<ToolUpdateResponse, String> {
    // First, check if the tool exists and get the necessary information
    let tool_info = {
        let registry = state.tool_registry.read().await;
        
        if let Some(tool) = registry.tools.get(&request.tool_id) {
            // Extract and clone the necessary values
            let tool_type = tool.get("tool_type")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
                
            let entry_point = tool.get("entry_point")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            
            let process_running = registry.processes.get(&request.tool_id).is_some_and(|p| p.is_some());
            
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
    
    // Unwrap the tool info
    let (tool_type, entry_point, process_running) = tool_info.unwrap();
    
    // Now handle the process based on the enabled status
    let mut registry = state.tool_registry.write().await;
    
    // Update the enabled status in the tool definition
    if let Some(tool) = registry.tools.get_mut(&request.tool_id) {
        if let Some(obj) = tool.as_object_mut() {
            obj.insert("enabled".to_string(), json!(request.enabled));
        }
    }
    
    // Handle process management
    if request.enabled {
        // Start process if it's not already running
        if !process_running {
            // Extract environment variables from the tool configuration
            let env_vars = if let Some(tool) = registry.tools.get(&request.tool_id) {
                if let Some(config) = tool.get("config") {
                    if let Some(env) = config.get("env") {
                        // Convert the JSON env vars to a HashMap<String, String>
                        let mut env_map = HashMap::new();
                        if let Some(env_obj) = env.as_object() {
                            for (key, value) in env_obj {
                                // Extract the value as a string
                                let value_str = match value {
                                    Value::String(s) => s.clone(),
                                    Value::Number(n) => n.to_string(),
                                    Value::Bool(b) => b.to_string(),
                                    _ => {
                                        // For objects, check if it has a description field (which means it's a template)
                                        if let Value::Object(obj) = value {
                                            if obj.contains_key("description") {
                                                // This is a template, so we don't have a value yet
                                                continue;
                                            }
                                        }
                                        // For other types, convert to JSON string
                                        value.to_string()
                                    }
                                };
                                env_map.insert(key.clone(), value_str);
                            }
                        }
                        Some(env_map)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };
            
            // Create a JSON Value with the command field from the entry_point string
            let config_value = json!({
                "command": entry_point
            });
            
            // Spawn process based on tool type
            let spawn_result = match tool_type.as_str() {
                "node" => {
                    info!("Spawning Node.js process for tool: {}", request.tool_id);
                    spawn_nodejs_process(&config_value, &request.tool_id, env_vars.as_ref()).await
                },
                "python" => {
                    info!("Spawning Python process for tool: {}", request.tool_id);
                    spawn_python_process(&config_value, &request.tool_id, env_vars.as_ref()).await
                },
                "docker" => {
                    info!("Spawning Docker process for tool: {}", request.tool_id);
                    spawn_docker_process(&config_value, &request.tool_id, env_vars.as_ref()).await
                },
                _ => {
                    info!("Unsupported tool type: {}", tool_type);
                    return Ok(ToolUpdateResponse {
                        success: false,
                        message: format!("Unsupported tool type: {}", tool_type),
                    });
                }
            };
            
            match spawn_result {
                Ok((process, stdin, stdout)) => {
                    registry.processes.insert(request.tool_id.clone(), Some(process));
                    registry.process_ios.insert(request.tool_id.clone(), (stdin, stdout));
                    
                    // Wait a moment for the server to start
                    drop(registry); // Release the lock during sleep
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    
                    // Try to discover tools from this server
                    let mut registry = state.tool_registry.write().await;
                    match discover_server_tools(&request.tool_id, &mut registry).await {
                        Ok(tools) => {
                            registry.server_tools.insert(request.tool_id.clone(), tools);
                        },
                        Err(e) => {
                            error!("Failed to discover tools from server {}: {}", request.tool_id, e);
                            // Continue even if discovery fails
                        }
                    }
                },
                Err(e) => {
                    return Ok(ToolUpdateResponse {
                        success: false,
                        message: format!("Failed to start process: {}", e),
                    });
                }
            }
        }
    } else {
        // Kill process if it's running
        if let Some(Some(process)) = registry.processes.get_mut(&request.tool_id) {
            if let Err(e) = kill_process(process).await {
                return Ok(ToolUpdateResponse {
                    success: false,
                    message: format!("Failed to kill process: {}", e),
                });
            }
            
            // Remove the process from the registry
            registry.processes.insert(request.tool_id.clone(), None);
            
            // Clear the server tools
            registry.server_tools.remove(&request.tool_id);
        }
    }
    
    // Return success
    Ok(ToolUpdateResponse {
        success: true,
        message: format!("Tool '{}' status updated to {}", request.tool_id, if request.enabled { "enabled" } else { "disabled" }),
    })
}

/// Update a tool's configuration (environment variables)
#[tauri::command]
pub async fn update_tool_config(
    state: State<'_, MCPState>,
    request: ToolConfigUpdateRequest,
) -> Result<ToolConfigUpdateResponse, String> {
    // First, check if the tool exists
    let tool_exists = {
        let registry = state.tool_registry.read().await;
        registry.tools.contains_key(&request.tool_id)
    };
    
    // If the tool doesn't exist, return an error
    if !tool_exists {
        return Ok(ToolConfigUpdateResponse {
            success: false,
            message: format!("Tool with ID '{}' not found", request.tool_id),
        });
    }
    
    // Update the tool configuration
    let mut registry = state.tool_registry.write().await;
    
    // Update the configuration in the tool definition
    if let Some(tool) = registry.tools.get_mut(&request.tool_id) {
        if let Some(obj) = tool.as_object_mut() {
            // Create or update the config object
            let config = obj.entry("config").or_insert(json!({}));
            
            if let Some(config_obj) = config.as_object_mut() {
                // Create or update the env object
                let env = config_obj.entry("env").or_insert(json!({}));
                
                if let Some(env_obj) = env.as_object_mut() {
                    // Update each environment variable
                    for (key, value) in &request.config.env {
                        env_obj.insert(key.clone(), json!(value));
                    }
                }
            }
        }
    }
    
    // Return success
    Ok(ToolConfigUpdateResponse {
        success: true,
        message: format!("Tool '{}' configuration updated", request.tool_id),
    })
}

/// Uninstall a registered tool
#[tauri::command]
pub async fn uninstall_tool(
    state: State<'_, MCPState>,
    request: ToolUninstallRequest,
) -> Result<ToolUninstallResponse, String> {
    let mut registry = state.tool_registry.write().await;
    
    // Kill the process if it's running
    if let Some(Some(process)) = registry.processes.get_mut(&request.tool_id) {
        if let Err(e) = kill_process(process).await {
            return Ok(ToolUninstallResponse {
                success: false,
                message: format!("Failed to kill process: {}", e),
            });
        }
    }
    
    // Remove the tool and process from the registry
    if registry.tools.remove(&request.tool_id).is_some() {
        registry.processes.remove(&request.tool_id);
        registry.server_tools.remove(&request.tool_id);
        
        Ok(ToolUninstallResponse {
            success: true,
            message: format!("Tool uninstalled successfully").to_string(),
        })
    } else {
        Ok(ToolUninstallResponse {
            success: false,
            message: format!("Tool with ID '{}' not found", request.tool_id),
        })
    }
}

/// Execute a registered tool
#[tauri::command]
pub async fn execute_tool(
    state: State<'_, MCPState>,
    request: ToolExecutionRequest,
) -> Result<ToolExecutionResponse, String> {
    // Shortcut to execute_proxy_tool using a direct tool ID
    let registry = state.tool_registry.read().await;
    
    // Check if the tool exists (fixed unused variable warning)
    if let Some(_) = registry.tools.get(&request.tool_id) {
        // Check if the process is running
        let process_running = registry.processes.get(&request.tool_id).is_some_and(|p| p.is_some());
        
        if !process_running {
            return Ok(ToolExecutionResponse {
                success: false,
                result: None,
                error: Some(format!("Tool with ID '{}' is not running", request.tool_id)),
            });
        }
        
        drop(registry);
        
        // Use the proxy format (server_id:tool_id)
        let proxy_request = ToolExecutionRequest {
            tool_id: format!("{}:main", request.tool_id),
            parameters: request.parameters,
        };
        
        // Execute the tool through the proxy
        execute_proxy_tool(state, proxy_request).await
    } else {
        Ok(ToolExecutionResponse {
            success: false,
            result: None,
            error: Some(format!("Tool with ID '{}' not found", request.tool_id)),
        })
    }
}

/// Get all server data in a single function to avoid multiple locks
#[tauri::command]
pub async fn get_all_server_data(state: State<'_, MCPState>) -> Result<Value, String> {
    // Acquire a single read lock for all operations
    let registry = state.tool_registry.read().await;
    
    // 1. Get registered servers
    let mut servers = Vec::new();
    for (id, tool_value) in registry.tools.iter() {
        // Clone the value so we can modify it
        let mut tool = tool_value.clone();
        
        // Ensure the tool has an ID field
        if let Some(obj) = tool.as_object_mut() {
            obj.insert("id".to_string(), json!(id));
            
            // Add process status
            let process_running = registry.processes.get(id).is_some_and(|p| p.is_some());
            obj.insert("process_running".to_string(), json!(process_running));
            
            // Add number of available tools from this server
            let server_tool_count = registry.server_tools.get(id).map_or_else(|| 0, |tools| tools.len());
            obj.insert("tool_count".to_string(), json!(server_tool_count));
        }
        
        servers.push(tool);
    }
    
    // 2. Get all server tools
    let mut all_tools = Vec::new();
    for (server_id, tools) in &registry.server_tools {
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
