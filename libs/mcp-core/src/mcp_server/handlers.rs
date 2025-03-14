use std::sync::Arc;
use async_trait::async_trait;
use serde_json::{Value};
use log::{error};
use std::future::Future;
use std::pin::Pin;
use tokio::runtime::Handle;

// Re-export types from mcp-server and mcp-core
pub use mcp_server::{
    RouterError
};
pub use mcp_core::{Tool, ToolCall};

/// Trait for client managers to implement
#[async_trait]
pub trait ClientManagerTrait: Send + Sync {
    /// Handle a tool call
    async fn handle_tool_call(&self, tool_name: String, arguments: Value) -> Result<Value, String>;
    
    /// List available tools
    async fn list_tools(&self) -> Result<Vec<Tool>, String>;
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
}

/// MCP Router implementation that uses a ClientManager
#[derive(Clone)]
struct MCPRouter {
    name: String,
    client_manager: Arc<dyn ClientManagerTrait>,
    capabilities: mcp_core::protocol::ServerCapabilities,
}

impl MCPRouter {
    fn new(name: String, client_manager: Arc<dyn ClientManagerTrait>) -> Self {
        // Create capabilities
        let mut builder = mcp_server::router::CapabilitiesBuilder::new();
        builder = builder.with_tools(true);
        
        Self {
            name,
            client_manager,
            capabilities: builder.build(),
        }
    }
}

impl mcp_server::Router for MCPRouter {
    fn name(&self) -> String {
        self.name.clone()
    }
    
    fn instructions(&self) -> String {
        "MCP Dockmaster Server".to_string()
    }
    
    fn capabilities(&self) -> mcp_core::protocol::ServerCapabilities {
        self.capabilities.clone()
    }
    
    fn list_tools(&self) -> Vec<Tool> {
        // Since list_tools is async but this method is sync, we need to block on the future
        // This is not ideal but necessary for the Router trait implementation
        let tools_future = self.client_manager.list_tools();
        
        // Get the current runtime handle and block on the future
        match Handle::try_current() {
            Ok(handle) => {
                match handle.block_on(tools_future) {
                    Ok(tools) => tools,
                    Err(err) => {
                        error!("Error listing tools: {}", err);
                        vec![]
                    }
                }
            },
            Err(_) => {
                error!("No tokio runtime available for listing tools");
                vec![]
            }
        }
    }
    
    fn call_tool(
        &self,
        tool_name: &str,
        arguments: Value,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<mcp_core::Content>, mcp_core::handler::ToolError>> + Send + 'static>> {
        let client_manager = self.client_manager.clone();
        let tool_name = tool_name.to_string();
        
        Box::pin(async move {
            match client_manager.handle_tool_call(tool_name, arguments).await {
                Ok(result) => {
                    // Convert the result to Content using the text method
                    let result_str = serde_json::to_string(&result).unwrap_or_default();
                    let content = mcp_core::Content::text(result_str);
                    Ok(vec![content])
                },
                Err(err) => Err(mcp_core::handler::ToolError::ExecutionError(err)),
            }
        })
    }
    
    fn list_resources(&self) -> Vec<mcp_core::Resource> {
        vec![]
    }
    
    fn read_resource(
        &self,
        _uri: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, mcp_core::handler::ResourceError>> + Send + 'static>> {
        Box::pin(async {
            Err(mcp_core::handler::ResourceError::NotFound("Resource not found".to_string()))
        })
    }
    
    fn list_prompts(&self) -> Vec<mcp_core::prompt::Prompt> {
        vec![]
    }
    
    fn get_prompt(
        &self,
        _prompt_name: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, mcp_core::handler::PromptError>> + Send + 'static>> {
        Box::pin(async {
            Err(mcp_core::handler::PromptError::NotFound("Prompt not found".to_string()))
        })
    }
}

/// Start the MCP server with the provided client manager
pub async fn start_mcp_server(client_manager: Arc<dyn ClientManagerTrait>) -> Result<(), Box<dyn std::error::Error>> {
    // Create our router implementation
    let router = MCPRouter::new("MCP Dockmaster Server".to_string(), client_manager);
    
    // Wrap it in a RouterService
    let router_service = mcp_server::router::RouterService(router);
    
    // Create the server
    let server = mcp_server::Server::new(router_service);
    
    // Create stdin/stdout transport for the MCP server
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let transport = mcp_server::ByteTransport::new(stdin, stdout);
    
    // Run the server
    server.run(transport).await?;
    
    Ok(())
} 