use std::time::Duration;

use log::info;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

use crate::MCPError;

/// Initialize a connection to an MCP server
pub async fn initialize_server_connection(
    server_id: &str,
    stdin: &mut tokio::process::ChildStdin,
    stdout: &mut tokio::process::ChildStdout,
) -> Result<(), MCPError> {
    info!("Initializing connection to server {}", server_id);

    let execute_cmd = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });

    let cmd_str = serde_json::to_string(&execute_cmd)
        .map_err(|e| MCPError::SerializationError(e.to_string()))?
        + "\n";

    info!("Command: {}", cmd_str.trim());

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

    let read_result =
        tokio::time::timeout(Duration::from_secs(1), reader.read_line(&mut response_line)).await;

    match read_result {
        Ok(Ok(0)) => return Err(MCPError::ServerClosedConnection),
        Ok(Ok(_)) => {}
        Ok(Err(e)) => return Err(MCPError::StdoutReadError(e.to_string())),
        Err(_) => return Err(MCPError::TimeoutError(server_id.to_string())),
    }

    Ok(())
}

/// Execute a tool on an MCP server
pub async fn execute_server_tool(
    server_id: &str,
    tool_name: &str,
    parameters: Value,
    stdin: &mut tokio::process::ChildStdin,
    stdout: &mut tokio::process::ChildStdout,
) -> Result<Value, MCPError> {
    let execute_cmd = json!({
        "jsonrpc": "2.0",
        "id": format!("execute_{}_{}", server_id, tool_name),
        "method": "tools/call",
        "params": { "name": tool_name, "arguments": parameters }
    });

    let cmd_str = serde_json::to_string(&execute_cmd)
        .map_err(|e| MCPError::SerializationError(e.to_string()))?
        + "\n";

    // Write command to stdin with better error handling
    match stdin.write_all(cmd_str.as_bytes()).await {
        Ok(_) => {}
        Err(e) => {
            // If the pipe is broken, the process might have died
            if e.kind() == std::io::ErrorKind::BrokenPipe {
                return Err(MCPError::StdinWriteError(format!(
                    "Process has died (broken pipe): {}",
                    e
                )));
            }
            return Err(MCPError::StdinWriteError(e.to_string()));
        }
    }

    // Flush stdin with better error handling
    match stdin.flush().await {
        Ok(_) => {}
        Err(e) => {
            // If the pipe is broken, the process might have died
            if e.kind() == std::io::ErrorKind::BrokenPipe {
                return Err(MCPError::StdinFlushError(format!(
                    "Process has died (broken pipe during flush): {}",
                    e
                )));
            }
            return Err(MCPError::StdinFlushError(e.to_string()));
        }
    };

    let mut reader = tokio::io::BufReader::new(&mut *stdout);
    let mut response_line = String::new();

    let read_result = tokio::time::timeout(
        Duration::from_secs(30),
        reader.read_line(&mut response_line),
    )
    .await;

    match read_result {
        Ok(Ok(0)) => return Err(MCPError::ServerClosedConnection),
        Ok(Ok(_)) => {}
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

/// MCPProxy: raw JSON-RPC & process-spawn operations
///
/// This module handles low-level operations for the MCP protocol:
/// - Spawning and killing child processes
/// - Sending JSON-RPC commands to child processes
/// - Reading JSON responses from child processes
/// - Discovering and executing tools via JSON-RPC
///
/// It focuses on I/O with child processes and doesn't directly manage
/// which tools are in the database or which tools are enabled.
/// Discover tools available from an MCP server
pub async fn discover_server_tools(
    server_id: &str,
    stdin: &mut tokio::process::ChildStdin,
    stdout: &mut tokio::process::ChildStdout,
) -> Result<Vec<Value>, String> {
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
        .map_err(|e| format!("Failed to serialize command: {}", e))?
        + "\n";

    info!("Command: {}", cmd_str.trim());

    // Write command to stdin
    match stdin.write_all(cmd_str.as_bytes()).await {
        Ok(_) => {}
        Err(e) => {
            // If the pipe is broken, the process might have died
            if e.kind() == std::io::ErrorKind::BrokenPipe {
                return Err(format!(
                    "Process for server {} has died (broken pipe)",
                    server_id
                ));
            }
            return Err(format!("Failed to write to process stdin: {}", e));
        }
    }

    match stdin.flush().await {
        Ok(_) => {}
        Err(e) => {
            // If the pipe is broken, the process might have died
            if e.kind() == std::io::ErrorKind::BrokenPipe {
                return Err(format!(
                    "Process for server {} has died (broken pipe during flush)",
                    server_id
                ));
            }
            return Err(format!("Failed to flush stdin: {}", e));
        }
    }

    // Read the response with a timeout
    let mut reader = tokio::io::BufReader::new(&mut *stdout);
    let mut response_line = String::new();

    let read_result = tokio::time::timeout(
        Duration::from_secs(10),
        reader.read_line(&mut response_line),
    )
    .await;

    match read_result {
        Ok(Ok(0)) => return Err("Server process closed connection".to_string()),
        Ok(Ok(_)) => info!(
            "Received response from server {}: {}",
            server_id,
            response_line.trim()
        ),
        Ok(Err(e)) => return Err(format!("Failed to read from process stdout: {}", e)),
        Err(_) => {
            return Err(format!(
                "Timeout waiting for response from server {}",
                server_id
            ))
        }
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
