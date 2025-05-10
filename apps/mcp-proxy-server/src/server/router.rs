// use std::{
//     future::Future,
//     pin::Pin,
//     sync::{Arc, Mutex},
// };

// use crate::server::mcp_client::McpClientProxy;
// use mcp_client::McpClientTrait;
// use mcp_core::{
//     Content, Resource, Tool, ToolError,
//     handler::{PromptError, ResourceError},
//     prompt::Prompt,
//     protocol::{JsonRpcNotification, ServerCapabilities},
// };
// use mcp_server::router::CapabilitiesBuilder;
// use serde_json::Value;

// pub struct DockmasterRouter {
//     tools_cache: Arc<Mutex<Vec<Tool>>>,
//     mcp_client: McpClientProxy,
// }

// impl Clone for DockmasterRouter {
//     fn clone(&self) -> Self {
//         Self {
//             tools_cache: self.tools_cache.clone(),
//             mcp_client: self.mcp_client.clone(),
//         }
//     }
// }

// impl Default for DockmasterRouter {
//     fn default() -> Self {
//         Self::new()
//     }
// }

// impl DockmasterRouter {
//     pub fn new() -> DockmasterRouter {
//         DockmasterRouter {
//             tools_cache: Arc::new(Mutex::new(vec![])),
//             mcp_client: McpClientProxy::new(),
//         }
//     }

//     pub async fn initialize(&mut self) {
//         // Create client instance
//         match self.mcp_client.init().await {
//             Ok(_) => {
//                 tracing::info!("Client initialized");
//             }
//             Err(e) => {
//                 tracing::error!("Error initializing client: {}", e);
//             }
//         }

//         // Save tools to cache
//         match self.mcp_client.get_client() {
//             Some(client) => {
//                 let tools = client.list_tools(None).await;
//                 if let Ok(mut cache) = self.tools_cache.lock() {
//                     *cache = tools.unwrap().tools;
//                 }
//             }
//             None => {
//                 tracing::error!("No client found");
//             }
//         }

//         // Spawn background task to update tools cache
//         match self.mcp_client.get_client() {
//             Some(client) => {
//                 let tools_cache = self.tools_cache.clone();
//                 tokio::spawn(async move {
//                     loop {
//                         match client.list_tools(None).await {
//                             Ok(list_tools_result) => {
//                                 if let Ok(mut cache) = tools_cache.lock() {
//                                     if list_tools_result.tools != *cache {
//                                         *cache = list_tools_result.tools;

//                                         // TODO: Send notification to clients
//                                         let _req = JsonRpcNotification {
//                                             method: "notifications/tools/list_changed".to_string(),
//                                             params: None,
//                                             jsonrpc: "2.0".to_string(),
//                                         };
//                                     }
//                                 }
//                             }
//                             Err(e) => {
//                                 tracing::error!("Error updating tools cache: {}", e);
//                             }
//                         }
//                         tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
//                     }
//                 });
//             }
//             None => {
//                 tracing::error!("No client found");
//             }
//         }
//     }

//     async fn execute_tool(
//         &self,
//         tool_name: &str,
//         arguments: Value,
//     ) -> Result<Vec<Content>, ToolError> {
//         tracing::info!(
//             "Executing tool: {} with arguments: {}",
//             tool_name,
//             arguments
//         );

//         match self.mcp_client.get_client() {
//             Some(client) => {
//                 let result = client.call_tool(tool_name, arguments).await;
//                 match result {
//                     Ok(response) => Ok(response.content),
//                     Err(e) => {
//                         tracing::error!("Error executing tool: {}", e);
//                         Err(ToolError::ExecutionError(e.to_string()))
//                     }
//                 }
//             }
//             None => {
//                 tracing::error!("No client found");
//                 Err(ToolError::ExecutionError("No client found".to_string()))
//             }
//         }
//     }
// }

// impl mcp_server::Router for DockmasterRouter {
//     fn list_tools(&self) -> Vec<Tool> {
//         // Use a synchronous lock to get the cached tools
//         self.tools_cache
//             .lock()
//             .map(|cache| cache.clone())
//             .unwrap_or_else(|_| {
//                 tracing::error!("Failed to acquire tools cache lock");
//                 vec![]
//             })
//     }

//     fn call_tool(
//         &self,
//         tool_name: &str,
//         arguments: Value,
//     ) -> Pin<Box<dyn Future<Output = Result<Vec<Content>, ToolError>> + Send + 'static>> {
//         let router = self.clone();
//         let tool_name = tool_name.to_string();
//         Box::pin(async move { router.execute_tool(&tool_name, arguments).await })
//     }

//     fn list_resources(&self) -> Vec<Resource> {
//         vec![]
//     }

//     fn read_resource(
//         &self,
//         uri: &str,
//     ) -> Pin<Box<dyn Future<Output = Result<String, ResourceError>> + Send + 'static>> {
//         let uri = uri.to_string();
//         Box::pin(async move {
//             Err(ResourceError::NotFound(format!(
//                 "Resource {} not found",
//                 uri
//             )))
//         })
//     }

//     fn list_prompts(&self) -> Vec<Prompt> {
//         vec![]
//     }

//     fn get_prompt(
//         &self,
//         prompt_name: &str,
//     ) -> Pin<Box<dyn Future<Output = Result<String, PromptError>> + Send + 'static>> {
//         let prompt_name = prompt_name.to_string();
//         Box::pin(async move {
//             Err(PromptError::NotFound(format!(
//                 "Prompt {} not found",
//                 prompt_name
//             )))
//         })
//     }
// }
