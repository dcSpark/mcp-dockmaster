use serde_json::json;

#[cfg(test)]
mod tests {
    use std::path;

    use mcp_core::{
        core::{mcp_core::MCPCore, mcp_core_proxy_ext::McpCoreProxyExt},
        event::LoggingEventEmitter,
        init_logging,
        models::types::ToolExecutionRequest,
        types::{ServerConfiguration, ServerRegistrationRequest},
    };

    use super::*;

    #[tokio::test]
    async fn test_mcp_core_with_registry() -> Result<(), String> {
        // Create a unique database path for this test
        use tempfile::tempdir;

        init_logging();
        let temp_dir = tempdir().map_err(|e| format!("Failed to create temp dir: {}", e))?;
        let db_path = temp_dir.path().join("test_mcp_core2.db");

        // TODO: Here the mcp-proxy-server path is a dummy path. This test currently is not using that location
        let mcp_core = MCPCore::new(db_path, path::absolute("mcp-proxy-server").unwrap());
        mcp_core.init().await.map_err(|e| {
            let message = format!("error initializing mcp core: {:?}", e);
            eprintln!("{}", message);
            message
        })?;

        // Get the absolute path to the script
        let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
        let script_path = current_dir
            .join("../../dist/apps/mcp-server-hello-world/index.js")
            .to_string_lossy()
            .into_owned();

        eprintln!("Script path: {}", script_path);

        let registration_request = ServerRegistrationRequest {
            server_id: "hello_world".to_string(),
            server_name: "Hello World".to_string(),
            description: "A simple hello world tool".to_string(),
            tools_type: "node".to_string(),
            configuration: Some(ServerConfiguration {
                command: Some("node".to_string()),
                args: Some(vec![
                    "--experimental-modules".to_string(),
                    "--no-warnings".to_string(),
                    script_path.clone(),
                ]),
                env: None,
            }),
            distribution: None,
        };

        eprintln!(
            "Registering tool with configuration: {:?}",
            registration_request
        );

        // Register tool
        let logging_emitter = LoggingEventEmitter;
        let response = mcp_core
            .register_server(logging_emitter, registration_request)
            .await?;
        let tool_id = response.tool_id.ok_or("No tool ID returned")?;

        eprintln!("Received tool_id from registration: {}", tool_id);

        // List all available tools
        let all_tools = mcp_core.list_all_server_tools().await?;
        eprintln!(
            "Available tools (list_all_server_tools): {}",
            serde_json::to_string_pretty(&all_tools).unwrap()
        );

        let all_tools_simple = mcp_core.list_servers().await?;
        eprintln!(
            "Available tools (list_servers): {}",
            serde_json::to_string_pretty(&all_tools_simple).unwrap()
        );

        // Execute tool
        let request = ToolExecutionRequest {
            tool_id: format!("{}:{}", tool_id, "hello_world"),
            parameters: json!({}),
        };

        let result = mcp_core.execute_proxy_tool(request).await?;

        // Print the execution result
        eprintln!(
            "Tool execution result: {}",
            serde_json::to_string_pretty(&result).unwrap()
        );

        // Verify result
        if !result.success {
            return Err(result.error.unwrap_or_else(|| "Unknown error".to_string()));
        }

        // Verify content matches expected
        let result_value = result.result.ok_or("No result found")?;
        let content = result_value
            .get("content")
            .and_then(|c| c.as_array())
            .ok_or("Content is not an array")?;

        if content.len() != 1 {
            return Err(format!("Expected 1 content item, got {}", content.len()));
        }

        let first_content = &content[0];
        let text = first_content
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or("Content text not found or not a string")?;

        if text != "hello world" {
            return Err(format!("Expected content 'hello world', got '{}'", text));
        }

        // Test hello_world_with_input
        let request = ToolExecutionRequest {
            tool_id: format!("{}:{}", tool_id, "hello_world_with_input"),
            parameters: json!({
                "message": "custom message"
            }),
        };

        let result = mcp_core.execute_proxy_tool(request).await?;

        eprintln!(
            "Tool execution result (with input): {}",
            serde_json::to_string_pretty(&result).unwrap()
        );

        if !result.success {
            return Err(result.error.unwrap_or_else(|| "Unknown error".to_string()));
        }

        let result_value = result.result.ok_or("No result found")?;
        let content = result_value
            .get("content")
            .and_then(|c| c.as_array())
            .ok_or("Content is not an array")?;

        if content.len() != 1 {
            return Err(format!("Expected 1 content item, got {}", content.len()));
        }

        let first_content = &content[0];
        let text = first_content
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or("Content text not found or not a string")?;

        if text != "hello world custom message" {
            return Err(format!(
                "Expected content 'hello world custom message', got '{}'",
                text
            ));
        }

        // Test hello_world_with_config
        let request = ToolExecutionRequest {
            tool_id: format!("{}:{}", tool_id, "hello_world_with_config"),
            parameters: json!({
                "config": "test-config"
            }),
        };

        let result = mcp_core.execute_proxy_tool(request).await?;

        eprintln!(
            "Tool execution result (with config): {}",
            serde_json::to_string_pretty(&result).unwrap()
        );

        if !result.success {
            return Err(result.error.unwrap_or_else(|| "Unknown error".to_string()));
        }

        let result_value = result.result.ok_or("No result found")?;
        let content = result_value
            .get("content")
            .and_then(|c| c.as_array())
            .ok_or("Content is not an array")?;

        if content.len() != 1 {
            return Err(format!("Expected 1 content item, got {}", content.len()));
        }

        let first_content = &content[0];
        let text = first_content
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or("Content text not found or not a string")?;

        if text != "hello configuration test-config" {
            return Err(format!(
                "Expected content 'hello configuration test-config', got '{}'",
                text
            ));
        }

        // Cleanup
        let _ = mcp_core.kill_all_processes().await;

        Ok(())
    }
}
