use rmcp::{
    model::{
        CallToolRequestParam, CallToolResult, Content, ErrorCode, ListToolsResult,
        PaginatedRequestParam, ServerCapabilities, ServerInfo, Tool,
    },
    service::RequestContext,
    tool, Error as McpError, RoleServer, ServerHandler,
};

use crate::{
    core::{mcp_core::MCPCore, mcp_core_proxy_ext::McpCoreProxyExt},
    types::ToolExecutionRequest,
};

use super::{
    get_configure_server_tool, get_register_server_tool, get_search_server_tool,
    tools::{
        get_list_installed_servers_tool, get_uninstall_server_tool, handle_configure_server,
        handle_list_installed_servers, handle_register_server, handle_search_server,
        handle_uninstall_server, TOOL_LIST_INSTALLED_SERVERS, TOOL_UNINSTALL_SERVER,
    },
    TOOL_CONFIGURE_SERVER, TOOL_REGISTER_SERVER, TOOL_SEARCH_SERVER,
};

pub struct McpServer {
    mcp_core: MCPCore,
}

#[tool(tool_box)]
impl McpServer {
    pub fn new(mcp_core: MCPCore) -> Self {
        Self { mcp_core }
    }
}

impl ServerHandler for McpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("Dockmaster MCP Server".into()),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_tool_list_changed()
                .enable_prompts()
                .enable_resources()
                .build(),
            ..Default::default()
        }
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        let server_tools = self.mcp_core.list_all_server_tools().await;
        let server_tools = server_tools.map_err(|e| {
            McpError::new(
                ErrorCode::INTERNAL_ERROR,
                "Failed to list tools",
                Some(e.into()),
            )
        })?;
        let tools: Vec<Tool> = vec![
            get_register_server_tool(),
            get_search_server_tool(),
            get_configure_server_tool(),
            get_list_installed_servers_tool(),
            get_uninstall_server_tool(),
        ]
        .into_iter()
        .chain(server_tools.into_iter().map(|tool| tool.to_tool().unwrap()))
        .collect();
        Ok(ListToolsResult {
            tools,
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        _: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        // let tool_id = request.name.into();
        match request.name.clone().to_string().as_str() {
            TOOL_REGISTER_SERVER => {
                handle_register_server(
                    self.mcp_core.clone(),
                    request.arguments.clone().unwrap_or_default(),
                )
                .await
            }
            TOOL_SEARCH_SERVER => handle_search_server(request.arguments.unwrap_or_default()).await,
            TOOL_CONFIGURE_SERVER => {
                handle_configure_server(
                    self.mcp_core.clone(),
                    request.arguments.unwrap_or_default(),
                )
                .await
            }
            TOOL_UNINSTALL_SERVER => {
                handle_uninstall_server(
                    self.mcp_core.clone(),
                    request.arguments.unwrap_or_default(),
                )
                .await
            }
            TOOL_LIST_INSTALLED_SERVERS => {
                handle_list_installed_servers(self.mcp_core.clone()).await
            }
            _ => {
                // For non-built-in tools, find the appropriate server that has this tool
                let mcp_state = self.mcp_core.mcp_state.read().await;
                let server_tools = mcp_state.server_tools.read().await;

                // Find which server has the requested tool
                let mut server_id = None;

                for (sid, tools) in &*server_tools {
                    for tool in tools {
                        if tool.name == request.name {
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
                            tool_id: format!("{}:{}", server_id, request.name),
                            parameters: request.arguments,
                        };

                        match self.mcp_core.execute_proxy_tool(request).await {
                            Ok(response) => {
                                if response.success {
                                    Ok(CallToolResult {
                                        content: vec![Content::text(
                                            serde_json::to_string(&response.result)
                                                .unwrap_or_default(),
                                        )],
                                        is_error: Some(false),
                                    })
                                } else {
                                    Err(McpError::new(
                                        ErrorCode::INTERNAL_ERROR,
                                        "Failed to execute tool",
                                        Some(serde_json::Value::String(
                                            response.error.unwrap_or("Unknown error".to_string()),
                                        )),
                                    ))
                                }
                            }
                            Err(e) => Err(McpError::new(
                                ErrorCode::INTERNAL_ERROR,
                                "Failed to execute tool",
                                Some(e.into()),
                            )),
                        }
                    }
                    None => Err(McpError::new(
                        ErrorCode::METHOD_NOT_FOUND,
                        format!("Tool '{}' not found", request.name),
                        None,
                    )),
                }
            }
        }
    }
}
