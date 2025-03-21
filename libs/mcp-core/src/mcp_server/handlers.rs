use std::sync::Arc;
use async_trait::async_trait;
use serde_json::{Value};
use std::future::Future;
use std::pin::Pin;
use log::error;

// Import SDK modules
use mcp_sdk_core::{Tool, Content, Resource};
use mcp_sdk_core::protocol;
use mcp_sdk_core::handler::{ToolError, ResourceError, PromptError};
use mcp_sdk_core::prompt;
use mcp_sdk_server::Router;

/// Trait for client managers to implement
#[async_trait]
pub trait ClientManagerTrait: Send + Sync {
    /// Handle a tool call
    async fn handle_tool_call(&self, tool_name: String, arguments: Value) -> Result<Value, String>;
    
    /// List available tools
    async fn list_tools(&self) -> Result<Vec<Tool>, String>;
    
    /// Synchronous version of list_tools for use in non-async contexts
    fn list_tools_sync(&self) -> Vec<Tool>;
    
    /// Update the tool cache
    async fn update_tools_cache(&self);
}

/// Default implementation of ClientManager
pub struct ClientManager {}

#[async_trait]
impl ClientManagerTrait for ClientManager {
    async fn handle_tool_call(&self, tool_name: String, _arguments: Value) -> Result<Value, String> {
        // Default implementation just returns an error
        Err(format!("Tool '{}' not implemented", tool_name))
    }
    
    async fn list_tools(&self) -> Result<Vec<Tool>, String> {
        // Default implementation returns an empty list
        Ok(vec![])
    }
    
    fn list_tools_sync(&self) -> Vec<Tool> {
        // Default implementation returns an empty list
        vec![]
    }
    
    async fn update_tools_cache(&self) {
        // Default implementation does nothing
    }
}

/// MCP Router implementation that uses a ClientManager
#[derive(Clone)]
pub struct MCPRouter {
    name: String,
    client_manager: Arc<dyn ClientManagerTrait>,
    capabilities: protocol::ServerCapabilities,
}

impl Router for MCPRouter {
    fn name(&self) -> String {
        self.name.clone()
    }
    
    fn instructions(&self) -> String {
        "This server provides access to MCP tools.".to_string()
    }
    
    fn capabilities(&self) -> protocol::ServerCapabilities {
        self.capabilities.clone()
    }
    
    fn list_tools(&self) -> Vec<Tool> {
        self.client_manager.list_tools_sync()
    }
    
    fn call_tool(
        &self,
        tool_name: &str,
        arguments: Value,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Content>, ToolError>> + Send + 'static>> {
        let client_manager = self.client_manager.clone();
        let tool_name = tool_name.to_string();
        
        Box::pin(async move {
            match client_manager.handle_tool_call(tool_name.clone(), arguments).await {
                Ok(result) => {
                    // Convert the result to a Content object
                    let json_result = serde_json::to_string_pretty(&result).unwrap_or_default();
                    Ok(vec![Content::text(json_result)])
                }
                Err(e) => {
                    error!("Tool call failed: {}", e);
                    Err(ToolError::ExecutionError(format!("Tool execution failed: {}", e)))
                }
            }
        })
    }
    
    fn list_resources(&self) -> Vec<Resource> {
        // We don't support resources yet
        vec![]
    }
    
    fn read_resource(
        &self,
        _uri: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, ResourceError>> + Send + 'static>> {
        Box::pin(async {
            Err(ResourceError::NotFound("Resources not supported".to_string()))
        })
    }
    
    fn list_prompts(&self) -> Vec<prompt::Prompt> {
        // We don't support prompts yet
        vec![]
    }
    
    fn get_prompt(
        &self,
        _prompt_name: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, PromptError>> + Send + 'static>> {
        Box::pin(async {
            Err(PromptError::NotFound("Prompts not supported".to_string()))
        })
    }
} 