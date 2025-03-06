use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;

#[cfg(test)]
mod tests {
  use mcp_core::{
        domain::traits::ProcessManager,
        application::dto::ToolExecutionDTO,
        application::services::tool_service::ToolService,
        infrastructure::process_management::mock_process_manager::MockProcessManager,
        infrastructure::repository::ToolRegistryRepository,
        mcp_state::MCPState,
        registry::ToolRegistry,
    };

    use super::*;

    #[tokio::test]
    async fn test_mcp_core_with_registry() -> Result<(), String> {
        // Initialize MCP state and services
        let tool_registry = Arc::new(RwLock::new(ToolRegistry::new()?));
        let mcp_state = MCPState {
            tool_registry: tool_registry.clone(),
        };
        
        // Create mock process manager and repository for testing
        let process_manager = Arc::new(MockProcessManager::new());
        let tool_repository = Arc::new(ToolRegistryRepository::new(tool_registry.clone(), process_manager.clone()));
        
        // Create tool service
        let tool_service = ToolService::new(tool_repository, process_manager.clone());

        // Get the absolute path to the script
        let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
        let script_path = current_dir
            .join("tests/integration/resources/mcp-server-hello-world/build/index.js")
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
        let registration_dto = serde_json::from_value(request).unwrap();
        let response = tool_service.register_tool(registration_dto).await
            .map_err(|e| e.to_string())?;
        let tool_id = response.tool_id.ok_or("No tool ID returned")?;

        eprintln!("Received tool_id from registration: {}", tool_id);

        // List all available tools
        let all_tools = tool_service.list_tools().await
            .map_err(|e| e.to_string())?;
        eprintln!(
            "Available tools: {}",
            serde_json::to_string_pretty(&all_tools).unwrap()
        );

        // Execute tool
        let request = ToolExecutionDTO {
            tool_id: tool_id.clone(),
            parameters: json!({
                "tool_name": "hello_world"
            }),
        };

        let result = tool_service.execute_tool(request).await
            .map_err(|e| e.to_string())?;

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
        let request = ToolExecutionDTO {
            tool_id: tool_id.clone(), // Use the tool_id directly, not as a server:tool format
            parameters: json!({
                "message": "custom message",
                "tool_name": "hello_world_with_input" // Specify the tool name in parameters
            }),
        };

        let result = tool_service.execute_tool(request).await
            .map_err(|e| e.to_string())?;

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
        let request = ToolExecutionDTO {
            tool_id: tool_id.clone(), // Use the tool_id directly, not as a server:tool format
            parameters: json!({
                "config": "test-config",
                "tool_name": "hello_world_with_config" // Specify the tool name in parameters
            }),
        };

        let result = tool_service.execute_tool(request).await
            .map_err(|e| e.to_string())?;

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
        let mut registry = mcp_state.tool_registry.write().await;
        registry.kill_all_processes().await;
        
        // Also clean up any processes managed by the process manager
        process_manager.kill_all_processes().await
            .map_err(|e| e.to_string())?;

        Ok(())
    }
}
