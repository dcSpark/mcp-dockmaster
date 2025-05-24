use std::{borrow::Cow, sync::Arc};

use log::error;
use rmcp::{
    model::{CallToolResult, Content, Tool},
    Error as McpError,
};
use serde::Deserialize;
use serde_json::{json, Map, Value};

use crate::{
    core::{mcp_core::MCPCore, mcp_core_proxy_ext::McpCoreProxyExt},
    mcp_server::registry_cache::fetch_tool_from_registry,
    registry::registry_search::{RegistrySearch, SearchError},
    types::{
        Distribution, ServerConfigUpdateRequest, ServerConfiguration, ServerRegistrationRequest,
        ToolUninstallRequest,
    },
};

#[derive(Deserialize, Debug)]
struct ToolRegistrationRequestByName {
    id: String,
    name: String,
    description: String,
    r#type: String,
    configuration: Option<ServerConfiguration>,
    distribution: Option<Distribution>,
}

#[derive(Deserialize, Debug)]
struct ToolRegistrationRequestById {
    tool_id: String,
}

#[derive(Deserialize, Debug)]
#[allow(clippy::large_enum_variant)]
#[serde(untagged)]
enum ToolRegistrationRequest {
    ByName(ToolRegistrationRequestByName),
    ById(ToolRegistrationRequestById),
}

#[derive(Deserialize, Debug)]
struct ServerSearchRequest {
    query: String,
}

/// Constants for tool names
pub const TOOL_REGISTER_SERVER: &str = "mcp_register_server";
pub const TOOL_SEARCH_SERVER: &str = "mcp_search_server";
pub const TOOL_CONFIGURE_SERVER: &str = "mcp_configure_server";
pub const TOOL_UNINSTALL_SERVER: &str = "mcp_uninstall_server";
pub const TOOL_LIST_INSTALLED_SERVERS: &str = "mcp_list_installed_servers";

/// Get the list installed servers tool definition
pub fn get_list_installed_servers_tool() -> Tool {
    Tool {
        name: Cow::Owned(TOOL_LIST_INSTALLED_SERVERS.to_string()),
        description: Some(Cow::Owned("List all installed servers".to_string())),
        input_schema: Arc::new(serde_json::Map::from_iter([
            ("type".to_string(), json!("object")),
            ("properties".to_string(), json!({})),
            ("required".to_string(), json!([])),
        ])),
        annotations: None,
    }
}

/// Get the register_server tool definition
pub fn get_register_server_tool() -> Tool {
    Tool {
        name: Cow::Owned(TOOL_REGISTER_SERVER.to_string()),
        description: Some(Cow::Owned(
            "Register a new server with MCP using its registry tool ID".to_string(),
        )),
        input_schema: Arc::new(serde_json::Map::from_iter([
            ("type".to_string(), json!("object")),
            (
                "properties".to_string(),
                json!({
                    "tool_id": {
                        "type": "string",
                        "description": "ID of the tool in the registry to install"
                    }
                }),
            ),
            ("required".to_string(), json!(["tool_id"])),
        ])),
        annotations: None,
    }
}

pub fn get_search_server_tool() -> Tool {
    Tool {
        name: Cow::Owned(TOOL_SEARCH_SERVER.to_string()),
        description: Some(Cow::Owned(
            "Search for MCP Servers in the registry".to_string(),
        )),
        input_schema: Arc::new(serde_json::Map::from_iter([
            ("type".to_string(), json!("object")),
            (
                "properties".to_string(),
                json!({
                    "query": {
                        "type": "string",
                        "description": "Search query"
                    }
                }),
            ),
            ("required".to_string(), json!(["query"])),
        ])),
        annotations: None,
    }
}

pub fn get_configure_server_tool() -> Tool {
    Tool {
        name: Cow::Owned(TOOL_CONFIGURE_SERVER.to_string()),
        description: Some(Cow::Owned(
            "Configure a server and its environment variables".to_string(),
        )),
        input_schema: Arc::new(serde_json::Map::from_iter([
            ("type".to_string(), json!("object")),
            (
                "properties".to_string(),
                json!({
                    "server_id": {
                        "type": "string",
                    "description": "ID of the server to configure"
                },
                "config": {
                    "type": "object",
                    "description": "Configuration for the server, it's not neccesary to nest the environment variables inside an env object, just pass the key and value"
                }
                }),
            ),
            ("required".to_string(), json!(["server_id", "config"])),
        ])),
        annotations: None,
    }
}

/// Get the uninstall_server tool definition
pub fn get_uninstall_server_tool() -> Tool {
    Tool {
        name: Cow::Owned(TOOL_UNINSTALL_SERVER.to_string()),
        description: Some(Cow::Owned(
            "Uninstall a server from MCP using its registry tool ID".to_string(),
        )),
        input_schema: Arc::new(serde_json::Map::from_iter([
            ("type".to_string(), json!("object")),
            (
                "properties".to_string(),
                json!({
                "server_id": {
                    "type": "string",
                    "description": "ID of the server to uninstall"
                    }
                }),
            ),
            ("required".to_string(), json!(["server_id"])),
        ])),
        annotations: None,
    }
}

pub async fn handle_register_server(
    mcp_core: MCPCore,
    params: Map<String, Value>,
) -> Result<CallToolResult, McpError> {
    let params: ToolRegistrationRequest =
        serde_json::from_value(serde_json::Value::Object(params)).unwrap();

    println!("[INSTALLATION] handle_register_tool: params {:?}", params);
    match params {
        ToolRegistrationRequest::ByName(request) => {
            println!(
                "[INSTALLATION] handle_register_tool: request (BY NAME) {:?}",
                request
            );
            let tool_id = request.id;
            let tool_name = request.name;
            let description = request.description;
            let tools_type = request.r#type;
            let configuration = request.configuration;
            let distribution = request.distribution;

            let tool = ServerRegistrationRequest {
                server_id: tool_id.clone(),
                server_name: tool_name,
                description,
                tools_type,
                configuration,
                distribution,
            };

            println!("[POST] handle_register_tool: tool {:?}", tool);
            let r = mcp_core.register_server(tool).await.map_err(|e| {
                McpError::internal_error(format!("Error registering server: {}", e), None)
            })?;
            println!("[INSTALLATION] handle_register_tool: r {:?}", r);
            Ok(CallToolResult {
                content: vec![Content::text(r.message)],
                is_error: Some(false),
            })
        }
        ToolRegistrationRequest::ById(request) => {
            println!(
                "[INSTALLATION] handle_register_tool: request (BY ID) {:?}",
                request
            );
            let tool_id = request.tool_id;

            let registry = fetch_tool_from_registry().await?;

            let tool = registry
                .tools
                .iter()
                .find(|tool| tool.id.as_str() == tool_id);
            if tool.is_none() {
                return Err(McpError::invalid_params(
                    format!("Tool {} not found", tool_id),
                    Some(serde_json::Value::String(tool_id)),
                ));
            }
            let tool = tool.unwrap();
            println!("Building tool from registry: {:?}", tool);
            let r = mcp_core
                .register_server(ServerRegistrationRequest {
                    server_id: tool_id.clone(),
                    server_name: tool.name.clone(),
                    description: tool.description.clone(),
                    tools_type: tool.runtime.clone(),
                    configuration: Some(tool.config.clone()),
                    distribution: Some(tool.distribution.clone()),
                })
                .await
                .map_err(|e| {
                    McpError::internal_error(format!("Error registering server: {}", e), None)
                })?;
            println!("[INSTALLATION] handle_register_tool: r {:?}", r);
            Ok(CallToolResult {
                content: vec![Content::text(r.message)],
                is_error: Some(false),
            })
        }
    }
}

/// Handle uninstall_server tool
pub async fn handle_uninstall_server(
    mcp_core: MCPCore,
    args: Map<String, Value>,
) -> Result<CallToolResult, McpError> {
    // Convert the args into the format expected by the HTTP handler
    let uninstall_request: ToolUninstallRequest =
        if let Some(server_id) = args.get("server_id").and_then(|v| v.as_str()) {
            ToolUninstallRequest {
                server_id: server_id.to_string(),
            }
        } else {
            // Otherwise, return error of missing server_id
            return Err(McpError::invalid_params(
                "Missing server_id parameter".to_string(),
                Some(serde_json::Value::String("server_id".to_string())),
            ));
        };
    match mcp_core.uninstall_server(uninstall_request).await {
        Ok(response) => Ok(CallToolResult {
            content: vec![Content::text(response.message)],
            is_error: Some(false),
        }),
        Err(error) => Err(McpError::internal_error(
            format!("Error uninstalling server: {}", error),
            None,
        )),
    }
}

/// Handle configure_server tool
pub async fn handle_configure_server(
    mcp_core: MCPCore,
    args: Map<String, Value>,
) -> Result<CallToolResult, McpError> {
    let configure_request: ServerConfigUpdateRequest =
        serde_json::from_value(serde_json::Value::Object(args)).unwrap();

    let r = mcp_core
        .update_server_config(configure_request)
        .await
        .map_err(|e| McpError::internal_error(format!("Error configuring server: {}", e), None))?;

    Ok(CallToolResult {
        content: vec![Content::text(r.message)],
        is_error: Some(false),
    })
}

/// Handle search_server tool
pub async fn handle_search_server(args: Map<String, Value>) -> Result<CallToolResult, McpError> {
    let params: ServerSearchRequest =
        serde_json::from_value(serde_json::Value::Object(args)).unwrap();

    // Create a new RegistrySearch instance
    let mut registry_search = match RegistrySearch::new().await {
        Ok(search) => search,
        Err(e) => match e {
            SearchError::CacheError(msg) => {
                error!("Cache error during registry search: {}", msg);
                return Err(McpError::internal_error(
                    format!("Cache error during registry search: {}", msg),
                    None,
                ));
            }
            SearchError::IndexError(msg) => {
                error!("Index error during registry search: {}", msg);
                return Err(McpError::internal_error(
                    format!("Registry index error: {}", msg),
                    None,
                ));
            }
            SearchError::QueryError(msg) => {
                error!("Query error during registry search: {}", msg);
                return Err(McpError::internal_error(
                    format!("Query error: {}", msg),
                    None,
                ));
            }
        },
    };

    // Execute the search
    let search_results = match registry_search.search(params.query.as_str()) {
        Ok(results) => results,
        Err(e) => match e {
            SearchError::QueryError(msg) => {
                return Err(McpError::invalid_params(
                    format!("Invalid query: {}", msg),
                    None,
                ));
            }
            _ => {
                return Err(McpError::invalid_params(
                    format!("Search execution error: {:?}", e),
                    None,
                ));
            }
        },
    };

    // Limit results to top 10 for better UI display
    let top_results = search_results.into_iter().take(10).collect::<Vec<_>>();

    // Transform results into a format suitable for JSON response
    let formatted_results = top_results
        .into_iter()
        .map(|(tool, score)| {
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
        })
        .collect::<Vec<_>>();

    // Return the results as JSON
    Ok(CallToolResult {
        content: vec![Content::text(
            serde_json::to_string(&formatted_results).unwrap(),
        )],
        is_error: Some(false),
    })
}

/// Handle list_installed_servers tool
pub async fn handle_list_installed_servers(mcp_core: MCPCore) -> Result<CallToolResult, McpError> {
    // Get the installed servers from MCPCore
    let result = mcp_core.list_servers().await;

    // Return the installed servers as JSON
    Ok(CallToolResult {
        content: vec![Content::text(serde_json::to_string(&result).unwrap())],
        is_error: Some(false),
    })
}
