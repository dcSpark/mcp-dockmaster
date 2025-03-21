use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

use crate::server::proxy::request;
use mcp_core::{
    Content, Resource, Tool, ToolError,
    handler::{PromptError, ResourceError},
    prompt::Prompt,
    protocol::ServerCapabilities,
};
use mcp_server::router::CapabilitiesBuilder;
use serde_json::Value;
use tracing;

#[derive(Clone)]
pub struct DockmasterRouter {
    tools_cache: Arc<Mutex<Vec<Tool>>>,
}

impl Default for DockmasterRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl DockmasterRouter {
    pub fn new() -> Self {
        Self {
            tools_cache: Arc::new(Mutex::new(vec![])),
        }
    }

    pub async fn initialize(&self) {
        // Fetch tools list and update cache
        let tools = Self::fetch_tools_list().await;
        if let Ok(mut cache) = self.tools_cache.lock() {
            *cache = tools;
        }

        // Spawn background task to update tools cache
        let tools_cache = self.tools_cache.clone();
        tokio::spawn(async move {
            loop {
                let tools = Self::fetch_tools_list().await;
                if let Ok(mut cache) = tools_cache.lock() {
                    *cache = tools;
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
            }
        });
    }

    async fn fetch_tools_list() -> Vec<Tool> {
        let request_result =
            request::<serde_json::Value, serde_json::Value>("tools/list", serde_json::json!({}))
                .await;

        match request_result {
            Ok(response) => {
                if let Some(tools) = response.get("tools") {
                    match serde_json::from_value(tools.clone()) {
                        Ok(tools) => tools,
                        Err(e) => {
                            tracing::error!("Error parsing tools: {}", e);
                            vec![]
                        }
                    }
                } else {
                    tracing::error!("No tools found in response");
                    vec![]
                }
            }
            Err(e) => {
                tracing::error!("Error making request: {}", e);
                vec![]
            }
        }
    }

    async fn execute_tool(tool_name: &str, arguments: Value) -> Result<Vec<Content>, ToolError> {
        tracing::info!(
            "Executing tool: {} with arguments: {}",
            tool_name,
            arguments
        );

        let request_result = request::<serde_json::Value, serde_json::Value>(
            "tools/call",
            serde_json::json!({
                "name": tool_name,
                "arguments": arguments
            }),
        )
        .await;

        match request_result {
            Ok(response) => {
                if let Some(content) = response.get("content") {
                    let mut contents = Vec::new();

                    if let Some(items) = content.as_array() {
                        for item in items {
                            if let Some(content_type) = item.get("type").and_then(|t| t.as_str()) {
                                match content_type {
                                    "text" => {
                                        if let Some(text) =
                                            item.get("text").and_then(|c| c.as_str())
                                        {
                                            contents.push(Content::text(text.to_string()));
                                        }
                                    }
                                    "image" => {
                                        if let (Some(data), Some(mime)) = (
                                            item.get("data").and_then(|d| d.as_str()),
                                            item.get("mimeType").and_then(|m| m.as_str()),
                                        ) {
                                            contents.push(Content::image(
                                                data.to_string(),
                                                mime.to_string(),
                                            ));
                                        }
                                    }
                                    _ => {
                                        tracing::warn!("Unknown content type: {}", content_type);
                                    }
                                }
                            }
                        }
                    }

                    if contents.is_empty() {
                        Err(ToolError::ExecutionError(
                            "No valid content found in response".to_string(),
                        ))
                    } else {
                        Ok(contents)
                    }
                } else {
                    // Handle single result as text for backward compatibility
                    let result_str = serde_json::to_string(&response)
                        .map_err(|e| ToolError::ExecutionError(e.to_string()))?;
                    Ok(vec![Content::text(result_str)])
                }
            }
            Err(e) => {
                tracing::error!("Error making request: {}", e);
                Err(ToolError::ExecutionError(e.to_string()))
            }
        }
    }
}

impl mcp_server::Router for DockmasterRouter {
    fn name(&self) -> String {
        "dockmaster".to_string()
    }

    fn instructions(&self) -> String {
        "This server is a proxy to the MCP Dockmaster Application.".to_string()
    }

    fn capabilities(&self) -> ServerCapabilities {
        CapabilitiesBuilder::new()
            .with_tools(true)
            .with_prompts(true)
            .with_resources(true, true)
            .build()
    }

    fn list_tools(&self) -> Vec<Tool> {
        // Use a synchronous lock to get the cached tools
        self.tools_cache
            .lock()
            .map(|cache| cache.clone())
            .unwrap_or_else(|_| {
                tracing::error!("Failed to acquire tools cache lock");
                vec![]
            })
    }

    fn call_tool(
        &self,
        tool_name: &str,
        arguments: Value,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Content>, ToolError>> + Send + 'static>> {
        let tool_name = tool_name.to_string();

        Box::pin(async move { Self::execute_tool(&tool_name, arguments).await })
    }

    fn list_resources(&self) -> Vec<Resource> {
        vec![]
    }

    fn read_resource(
        &self,
        uri: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, ResourceError>> + Send + 'static>> {
        let uri = uri.to_string();
        Box::pin(async move {
            Err(ResourceError::NotFound(format!(
                "Resource {} not found",
                uri
            )))
        })
    }

    fn list_prompts(&self) -> Vec<Prompt> {
        vec![]
    }

    fn get_prompt(
        &self,
        prompt_name: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, PromptError>> + Send + 'static>> {
        let prompt_name = prompt_name.to_string();
        Box::pin(async move {
            Err(PromptError::NotFound(format!(
                "Prompt {} not found",
                prompt_name
            )))
        })
    }
}
