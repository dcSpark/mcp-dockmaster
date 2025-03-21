use std::{future::Future, pin::Pin, sync::Arc};

use mcp_sdk_core::{
    handler::{PromptError, ResourceError},
    prompt::Prompt,
    protocol::ServerCapabilities,
    Content, Resource, Tool, ToolError,
};
use mcp_sdk_server::router::CapabilitiesBuilder;
use serde_json::{json, Value};
use tokio::sync::RwLock;
use log::{info, error};

use crate::{
    core::mcp_core::MCPCore,
    core::mcp_core_proxy_ext::McpCoreProxyExt,
    models::types::{ServerToolsResponse, ToolExecutionRequest},
    registry::registry_search::{RegistrySearch, SearchError},
};

use super::tools::{
    TOOL_REGISTER_SERVER, get_register_server_tool,
    TOOL_SEARCH_SERVER, get_search_server_tool,
};

/// MCP Router implementation for the Dockmaster server
/// This router handles all MCP protocol methods and integrates with the MCPCore
#[derive(Clone)]
pub struct MCPDockmasterRouter {
    mcp_core: MCPCore,
    server_name: String,
    version: String,
    tools_cache: Arc<RwLock<Vec<Tool>>>,
}

impl MCPDockmasterRouter {
    /// Create a new MCP router for the Dockmaster server
    pub fn new(mcp_core: MCPCore) -> Self {
        Self {
            mcp_core,
            server_name: "mcp-dockmaster-server".to_string(),
            version: "1.0.0".to_string(),
            tools_cache: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Get all server tools
    async fn list_all_tools(&self) -> Result<Vec<Tool>, ToolError> {
        // Check the cache first
        {
            let cache = self.tools_cache.read().await;
            if !cache.is_empty() {
                return Ok(cache.clone());
            }
        }

        // Get user-installed tools from MCPCore
        match self.get_server_tools().await {
            Ok(response) => {
                let mut tools = Vec::new();
                
                // Add built-in tools
                tools.push(get_register_server_tool());
                tools.push(get_search_server_tool());

                // Add user-installed tools
                for tool_info in response.tools {
                    // Convert ServerToolInfo to Tool
                    if let Some(input_schema) = tool_info.input_schema {
                        // Convert InputSchema to serde_json::Value
                        let schema_value = json!({
                            "type": input_schema.r#type,
                            "properties": input_schema.properties,
                            "required": input_schema.required,
                        });
                        
                        let tool = Tool {
                            name: tool_info.name,
                            description: tool_info.description,
                            input_schema: schema_value,
                        };
                        
                        tools.push(tool);
                    } else {
                        // Create a tool with an empty schema
                        let tool = Tool {
                            name: tool_info.name,
                            description: tool_info.description,
                            input_schema: json!({
                                "type": "object",
                                "properties": {},
                                "required": []
                            }),
                        };
                        
                        tools.push(tool);
                    }
                }
                
                // Update the cache
                {
                    let mut cache = self.tools_cache.write().await;
                    *cache = tools.clone();
                }
                
                Ok(tools)
            },
            Err(error) => Err(ToolError::NotFound(format!("Failed to list tools: {}", error.message))),
        }
    }
    
    /// Get server tools using MCPCore
    async fn get_server_tools(&self) -> Result<ServerToolsResponse, crate::models::types::ErrorResponse> {
        // Get the installed tools from MCPCore
        let result = self.mcp_core.list_all_server_tools().await;

        match result {
            Ok(tools) => {
                // Use the existing ServerToolsResponse struct
                Ok(ServerToolsResponse {
                    tools: tools,
                })
            },
            Err(e) => Err(crate::models::types::ErrorResponse {
                code: -32000,
                message: format!("Failed to list tools: {}", e),
            }),
        }
    }
    
    /// Handle register_server tool
    async fn handle_register_server(&self, args: Value) -> Result<Value, ToolError> {
        // Convert the args into the format expected by the HTTP handler
        let registration_request = if let Some(tool_id) = args.get("tool_id").and_then(|v| v.as_str()) {
            // If we receive a tool_id, treat it as a registry-based installation
            serde_json::json!({
                "tool_id": tool_id
            })
        } else {
            // Otherwise, expect direct registration parameters
            args
        };

        // Use the HTTP handler's logic through MCPCore
        match crate::http_server::handlers::handle_register_tool(
            self.mcp_core.clone(),
            registration_request
        ).await {
            Ok(response) => {
                // Update the tools cache after successful registration
                self.update_tools_cache().await;
                
                // Spawn the broadcast notification as a separate task
                tokio::spawn(async {
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    crate::mcp_server::notifications::broadcast_tools_list_changed().await;
                });
                
                Ok(json!({
                    "success": true,
                    "message": response.message,
                    "tool_id": response.tool_id
                }))
            },
            Err(error) => Err(ToolError::ExecutionError(error.message)),
        }
    }

    /// Handle search_server tool
    async fn handle_search_server(&self, args: Value) -> Result<Value, ToolError> {
        // Extract the query parameter from the args
        let query = match args.get("query").and_then(|q| q.as_str()) {
            Some(q) => q,
            None => return Err(ToolError::ExecutionError("Missing or invalid 'query' parameter".to_string())),
        };

        // Create a new RegistrySearch instance
        let mut registry_search = match RegistrySearch::new().await {
            Ok(search) => search,
            Err(e) => match e {
                SearchError::CacheError(msg) => {
                    error!("Cache error during registry search: {}", msg);
                    return Err(ToolError::ExecutionError(format!("Registry cache error: {}", msg)));
                },
                SearchError::IndexError(msg) => {
                    error!("Index error during registry search: {}", msg);
                    return Err(ToolError::ExecutionError(format!("Registry index error: {}", msg)));
                },
                SearchError::QueryError(msg) => {
                    error!("Query error during registry search: {}", msg);
                    return Err(ToolError::ExecutionError(format!("Query error: {}", msg)));
                },
            },
        };

        // Execute the search
        let search_results = match registry_search.search(query) {
            Ok(results) => results,
            Err(e) => match e {
                SearchError::QueryError(msg) => {
                    return Err(ToolError::ExecutionError(format!("Invalid query: {}", msg)));
                },
                _ => {
                    return Err(ToolError::ExecutionError(format!("Search execution error: {:?}", e)));
                },
            },
        };

        // Limit results to top 10 for better UI display
        let top_results = search_results.into_iter().take(10).collect::<Vec<_>>();
        
        // Transform results into a format suitable for JSON response
        let formatted_results = top_results.into_iter().map(|(tool, score)| {
            json!({
                "id": tool.id,
                "name": tool.name,
                "description": tool.description,
                "short_description": tool.short_description,
                "publisher": tool.publisher,
                "is_official": tool.is_official,
                "source_url": tool.source_url,
                "distribution": tool.distribution,
                "license": tool.license,
                "runtime": tool.runtime,
                "categories": tool.categories,
                "tags": tool.tags,
                "score": score
            })
        }).collect::<Vec<_>>();

        // Return the results as JSON
        Ok(json!({
            "results": formatted_results,
            "count": formatted_results.len(),
            "query": query
        }))
    }

    /// Execute a tool by finding the appropriate server and forwarding the call
    async fn execute_tool(&self, tool_name: &str, args: Value) -> Result<Value, ToolError> {
        // Handle built-in tools first
        if tool_name == TOOL_REGISTER_SERVER {
            return self.handle_register_server(args).await;
        }

        if tool_name == TOOL_SEARCH_SERVER {
            return self.handle_search_server(args).await;
        }
        
        // For non-built-in tools, find the appropriate server that has this tool
        let mcp_state = self.mcp_core.mcp_state.read().await;
        let server_tools = mcp_state.server_tools.read().await;

        // Find which server has the requested tool
        let mut server_id = None;

        for (sid, tools) in &*server_tools {
            for tool in tools {
                if tool.id == tool_name {
                    server_id = Some(sid.clone());
                    break;
                }

                // Also check by name if id doesn't match
                if tool.name == tool_name {
                    server_id = Some(sid.clone());
                    break;
                }
            }

            if server_id.is_some() {
                break;
            }
        }

        // Drop the locks before proceeding
        drop(server_tools);
        drop(mcp_state);

        match server_id {
            Some(server_id) => {
                let request = ToolExecutionRequest {
                    tool_id: format!("{}:{}", server_id, tool_name),
                    parameters: args,
                };

                match self.mcp_core.execute_proxy_tool(request).await {
                    Ok(response) => {
                        if response.success {
                            Ok(response.result.unwrap_or(json!(null)))
                        } else {
                            Err(ToolError::ExecutionError(response.error.unwrap_or_else(|| "Unknown error".to_string())))
                        }
                    },
                    Err(e) => Err(ToolError::ExecutionError(format!("Failed to execute tool: {}", e))),
                }
            },
            None => Err(ToolError::NotFound(format!("Tool '{}' not found", tool_name))),
        }
    }
    
    /// Update the tools cache with the latest tools from all servers
    pub async fn update_tools_cache(&self) {
        // Update the tools cache
        match self.list_all_tools().await {
            Ok(tools) => {
                let mut cache = self.tools_cache.write().await;
                *cache = tools.clone();
                log::info!("Tools cache updated with {} tools", cache.len());
            },
            Err(e) => {
                log::error!("Failed to update tools cache: {}", e);
                // Clear the cache to force refresh on next request
                let mut cache = self.tools_cache.write().await;
                cache.clear();
            }
        }
    }
}

impl mcp_sdk_server::Router for MCPDockmasterRouter {
    fn name(&self) -> String {
        self.server_name.clone()
    }

    fn instructions(&self) -> String {
        "This server provides tools for managing Docker containers, images, and networks. You can use it to manage containers, build images, and interact with Docker registries. It also allows you to register new MCP servers.".to_string()
    }

    fn capabilities(&self) -> ServerCapabilities {
        // Build capabilities with tools support
        CapabilitiesBuilder::new()
            .with_tools(false)
            .with_resources(false, false)
            .with_prompts(false)
            .build()
    }

    fn list_tools(&self) -> Vec<Tool> {
        // This is synchronous, so we need to use a synchronous approach instead of block_on
        // Start with an empty list of tools
        let mut tools = Vec::new();
        
        // Get cached tools if available (using a std::sync Mutex instead of an async lock)
        let cache_handle = self.tools_cache.clone();
        
        // Try to read from the existing cache (non-blocking)
        let cache_found = if let Ok(cache) = cache_handle.try_read() {
            if !cache.is_empty() {
                log::info!("Found {} tools in cache", cache.len());
                // Add all cached tools
                tools = cache.clone();
                true
            } else {
                false
            }
        } else {
            false
        };
        
        // If we didn't find any cached tools, add the register_server tool
        if !cache_found {
            log::info!("No tools in cache, adding register_server tool");
            tools.push(get_register_server_tool());
            tools.push(get_search_server_tool());
        }
        
        // Log what we're returning
        log::info!("Returning {} tools from list_tools", tools.len());
        
        // Trigger an async task to update the cache for future calls
        let mcp_core = self.mcp_core.clone();
        let cache_clone = cache_handle.clone();
        
        // Spawn a task to update the cache for future requests
        tokio::spawn(async move {
            match mcp_core.list_all_server_tools().await {
                Ok(server_tools) => {
                    let mut tools_vec = Vec::new();
                    
                    // Add built-in tools
                    tools_vec.push(get_register_server_tool());
                    tools_vec.push(get_search_server_tool());

                    // Add user-installed tools
                    for tool_info in server_tools {
                        // Convert ServerToolInfo to Tool
                        if let Some(input_schema) = tool_info.input_schema {
                            // Convert InputSchema to serde_json::Value
                            let schema_value = json!({
                                "type": input_schema.r#type,
                                "properties": input_schema.properties,
                                "required": input_schema.required,
                            });
                            
                            let tool = Tool {
                                name: tool_info.name,
                                description: tool_info.description,
                                input_schema: schema_value,
                            };
                            
                            tools_vec.push(tool);
                        } else {
                            // Create a tool with an empty schema
                            let tool = Tool {
                                name: tool_info.name,
                                description: tool_info.description,
                                input_schema: json!({
                                    "type": "object",
                                    "properties": {},
                                    "required": []
                                }),
                            };
                            
                            tools_vec.push(tool);
                        }
                    }
                    
                    // Update the cache
                    let mut cache = cache_clone.write().await;
                    *cache = tools_vec;
                    log::info!("Tools cache updated with {} tools", cache.len());
                },
                Err(e) => {
                    log::error!("Failed to update tools cache: {}", e);
                }
            }
        });
        
        tools
    }

    fn call_tool(
        &self,
        tool_name: &str,
        arguments: Value,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Content>, ToolError>> + Send + 'static>> {
        let this = self.clone();
        let tool_name = tool_name.to_string();
        info!("Calling tool: {}", tool_name);
        Box::pin(async move {
            match this.execute_tool(&tool_name, arguments).await {
                Ok(result) => {
                    let result_str = serde_json::to_string_pretty(&result).unwrap_or_default();
                    Ok(vec![Content::text(result_str)])
                },
                Err(e) => Err(e),
            }
        })
    }

    fn list_resources(&self) -> Vec<Resource> {
        // No resources for now
        vec![]
    }

    fn read_resource(
        &self,
        uri: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, ResourceError>> + Send + 'static>> {
        let uri = uri.to_string();
        Box::pin(async move {
            Err(ResourceError::NotFound(format!("Resource not found: {}", uri)))
        })
    }

    fn list_prompts(&self) -> Vec<Prompt> {
        // No prompts for now
        vec![]
    }

    fn get_prompt(
        &self,
        prompt_name: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, PromptError>> + Send + 'static>> {
        let prompt_name = prompt_name.to_string();
        Box::pin(async move {
            Err(PromptError::NotFound(format!("Prompt not found: {}", prompt_name)))
        })
    }
}
