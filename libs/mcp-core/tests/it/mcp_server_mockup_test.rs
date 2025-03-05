use mcp_core::error::{MCPError, MCPResult};
use mcp_core::mcp_proxy::{self, MCPState, ToolRegistry};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::{Child, Command};
use tokio::sync::RwLock;
use tokio::time::sleep;

#[cfg(test)]
mod tests {
    use super::*;

    /// A wrapper for the hello world MCP server
    struct HelloWorldServer {
        child: Child,
    }

    impl HelloWorldServer {
        /// Start the hello world server
        async fn new() -> MCPResult<Self> {
            // Navigate to the hello world server directory and start it
            let child = Command::new("npm")
                .arg("run")
                .arg("start")
                .current_dir("libs/mcp-core/tests/it/resources/mcp-server-hello-world")
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()?;

            // Return the server wrapper
            Ok(Self { child })
        }

        /// Clean up resources
        async fn cleanup(&mut self) -> MCPResult<()> {
            if let Err(e) = self.child.kill().await {
                eprintln!("Failed to kill hello world server: {}", e);
            }
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_mcp_core_with_registry() -> Result<(), String> {
        // Initialize MCP state
        let mcp_state = MCPState {
            tool_registry: Arc::new(RwLock::new(ToolRegistry::default())),
        };

        // Start the hello world server
        let mut server = HelloWorldServer::new().await.map_err(|e| e.to_string())?;

        // Create registration request
        let request = json!({
            "tool_name": "hello_world_with_input",
            "description": "A simple hello world tool with input",
            "tool_type": "node",
            "authentication": null,
            "configuration": {
                "command": "node",
                "args": ["build/index.js"]
            },
            "distribution": null
        });

        // Register tool
        let response =
            mcp_proxy::register_tool(&mcp_state, serde_json::from_value(request).unwrap()).await?;
        let tool_id = response.tool_id.ok_or("No tool ID returned")?;

        // Wait for initialization
        sleep(Duration::from_secs(2)).await;

        // Execute tool
        let exec_request = json!({
            "tool_id": tool_id,
            "parameters": {"message": "Test"}
        });
        let result =
            mcp_proxy::execute_tool(&mcp_state, serde_json::from_value(exec_request).unwrap())
                .await?;

        // Verify result
        if !result.success {
            return Err(result.error.unwrap_or_else(|| "Unknown error".to_string()));
        }

        // Cleanup
        server.cleanup().await.map_err(|e| e.to_string())?;
        let mut registry = mcp_state.tool_registry.write().await;
        registry.kill_all_processes().await;

        Ok(())
    }
}
