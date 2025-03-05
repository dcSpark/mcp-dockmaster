use mcp_core::error::{MCPError, MCPResult};
use mcp_core::mcp_proxy::{self, MCPState, ToolRegistry};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::RwLock;
use tokio::time::sleep;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mcp_core_with_registry() -> Result<(), String> {
        // Initialize MCP state
        let mcp_state = MCPState {
            tool_registry: Arc::new(RwLock::new(ToolRegistry::default())),
        };

        // Get the absolute path to the script
        let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
        let script_path = current_dir
            .join("tests/it/resources/mcp-server-hello-world/build/index.js")
            .to_string_lossy()
            .into_owned();

        eprintln!("Script path: {}", script_path);

        // Create registration request
        let request = json!({
            "tool_name": "hello_world",
            "description": "A simple hello world tool",
            "tool_type": "node",
            "authentication": null,
            "configuration": {
                "command": "node",
                "args": ["--experimental-modules", "--no-warnings", script_path]
            },
            "distribution": null
        });

        eprintln!(
            "Registering tool with configuration: {}",
            serde_json::to_string_pretty(&request).unwrap()
        );

        // Register tool
        let response =
            mcp_proxy::register_tool(&mcp_state, serde_json::from_value(request).unwrap()).await?;
        let tool_id = response.tool_id.ok_or("No tool ID returned")?;

        // Execute tool
        let exec_request = json!({
            "tool_id": tool_id,
            "parameters": {"name": "Test"}
        });
        let result =
            mcp_proxy::execute_tool(&mcp_state, serde_json::from_value(exec_request).unwrap())
                .await?;

        // Verify result
        if !result.success {
            return Err(result.error.unwrap_or_else(|| "Unknown error".to_string()));
        }

        // Cleanup
        let mut registry = mcp_state.tool_registry.write().await;
        registry.kill_all_processes().await;

        Ok(())
    }
}
