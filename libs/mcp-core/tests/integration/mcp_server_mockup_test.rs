use serde_json::json;

#[cfg(test)]
mod tests {
    use std::path;

    use mcp_core::{
        core::{mcp_core::MCPCore, mcp_core_proxy_ext::McpCoreProxyExt},
        init_logging,
        models::types::ToolExecutionRequest,
        types::{ServerConfiguration, ServerRegistrationRequest, ServerUpdateRequest},
    };
    use serde_json::Map;

    use super::*;

    #[tokio::test]
    async fn test_mcp_core_with_registry() -> Result<(), String> {
        // Create a unique database path for this test
        use tempfile::tempdir;

        init_logging();
        let temp_dir = tempdir().map_err(|e| format!("Failed to create temp dir: {}", e))?;
        let db_path = temp_dir.path().join("test_mcp_core2.db");

        // TODO: Here the mcp-proxy-server path is a dummy path. This test currently is not using that location
        let mcp_core = MCPCore::new(
            db_path,
            path::absolute("mcp-proxy-server").unwrap(),
            "mcp-core-test".to_string(),
        );
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
        let response = mcp_core.register_server(registration_request).await?;
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
            parameters: None,
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
        let mut parameters = Map::new();
        parameters.insert("message".to_string(), json!("custom message"));
        let request = ToolExecutionRequest {
            tool_id: format!("{}:{}", tool_id, "hello_world_with_input"),
            parameters: Some(parameters),
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
        let mut parameters = Map::new();
        parameters.insert("config".to_string(), json!("test-config"));
        let request = ToolExecutionRequest {
            tool_id: format!("{}:{}", tool_id, "hello_world_with_config"),
            parameters: Some(parameters),
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

    #[tokio::test]
    async fn test_mcp_core_with_registry_legacy() -> Result<(), String> {
        // Create a unique database path for this test
        use tempfile::tempdir;

        init_logging();
        let temp_dir = tempdir().map_err(|e| format!("Failed to create temp dir: {}", e))?;
        let db_path = temp_dir.path().join("test_mcp_core2.db");

        // TODO: Here the mcp-proxy-server path is a dummy path. This test currently is not using that location
        let mcp_core = MCPCore::new_with_port(
            db_path,
            path::absolute("mcp-proxy-server").unwrap(),
            3002,
            "mcp-core-test".to_string(),
        );
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
        let response = mcp_core.register_server(registration_request).await?;
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
        let mut parameters = Map::new();
        parameters.insert("format".to_string(), json!("legacy"));
        let request = ToolExecutionRequest {
            tool_id: format!("{}:{}", tool_id, "hello_world"),
            parameters: Some(parameters),
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
            .get("toolResult")
            .and_then(|c| c.get("content"))
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
        let mut parameters = Map::new();
        parameters.insert("message".to_string(), json!("custom message"));
        parameters.insert("format".to_string(), json!("legacy"));
        let request = ToolExecutionRequest {
            tool_id: format!("{}:{}", tool_id, "hello_world_with_input"),
            parameters: Some(parameters),
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
            .get("toolResult")
            .and_then(|c| c.get("content"))
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
        let mut parameters = Map::new();
        parameters.insert("config".to_string(), json!("test-config"));
        parameters.insert("format".to_string(), json!("legacy"));
        let request = ToolExecutionRequest {
            tool_id: format!("{}:{}", tool_id, "hello_world_with_config"),
            parameters: Some(parameters),
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
            .get("toolResult")
            .and_then(|c| c.get("content"))
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

    #[tokio::test]
    async fn test_server_lifecycle_management() -> Result<(), String> {
        // Create a unique database path for this test
        use tempfile::tempdir;

        init_logging();
        let temp_dir = tempdir().map_err(|e| format!("Failed to create temp dir: {}", e))?;
        let db_path = temp_dir.path().join("test_server_lifecycle.db");

        // Initialize MCP Core
        let mcp_core = MCPCore::new_with_port(
            db_path,
            path::absolute("mcp-proxy-server").unwrap(),
            3003,
            "mcp-core-test".to_string(),
        );
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

        // Register a new server
        let registration_request = ServerRegistrationRequest {
            server_id: "lifecycle_test".to_string(),
            server_name: "Lifecycle Test Server".to_string(),
            description: "A server for testing lifecycle management".to_string(),
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

        // Register the server
        let response = mcp_core.register_server(registration_request).await?;
        let server_id = response.tool_id.ok_or("No server ID returned")?;

        // Verify server is running by executing a command
        let mut parameters = Map::new();
        let request = ToolExecutionRequest {
            tool_id: format!("{}:{}", server_id, "hello_world"),
            parameters: Some(parameters),
        };
        let result = mcp_core.execute_proxy_tool(request).await?;
        assert!(result.success, "Initial server execution failed");

        // Stop the server
        let stop_request = ServerUpdateRequest {
            server_id: server_id.clone(),
            enabled: false,
        };
        let stop_result = mcp_core.update_server_status(stop_request).await?;
        assert!(stop_result.success, "Failed to stop server");

        // Poll for up to 5 seconds to verify the server has stopped
        let start_time = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(10);
        let check_interval = std::time::Duration::from_millis(500);

        loop {
            let parameters = Map::new();
            let request = ToolExecutionRequest {
                tool_id: format!("{}:{}", server_id, "hello_world"),
                parameters: Some(parameters),
            };

            match mcp_core.execute_proxy_tool(request).await {
                Ok(_) => {
                    // Server is still running
                    if start_time.elapsed() >= timeout {
                        return Err("Server did not stop within 5 seconds timeout".to_string());
                    }
                    tokio::time::sleep(check_interval).await;
                    continue;
                }
                Err(_) => {
                    // Server has stopped
                    println!(
                        "Server successfully stopped after {:?}",
                        start_time.elapsed()
                    );
                    break;
                }
            }
        }

        // Verify server is stopped by attempting to execute a command (one final check)
        let parameters = Map::new();
        let request = ToolExecutionRequest {
            tool_id: format!("{}:{}", server_id, "hello_world"),
            parameters: Some(parameters),
        };
        let result = mcp_core.execute_proxy_tool(request).await;
        assert!(
            result.is_err(),
            "Server should be stopped but is still responding"
        );

        // Restart the server
        let start_request = ServerUpdateRequest {
            server_id: server_id.clone(),
            enabled: true,
        };
        let start_result = mcp_core.update_server_status(start_request).await?;
        assert!(start_result.success, "Failed to restart server");

        // Verify server is running again
        let parameters = Map::new();
        let request = ToolExecutionRequest {
            tool_id: format!("{}:{}", server_id, "hello_world"),
            parameters: Some(parameters),
        };
        let result = mcp_core.execute_proxy_tool(request).await?;
        assert!(result.success, "Server did not restart successfully");

        // Verify the content of the response
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

        // Cleanup
        let _ = mcp_core.kill_all_processes().await;

        Ok(())
    }
}
