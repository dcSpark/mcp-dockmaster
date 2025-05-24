use rmcp::{
    Error as McpError, RoleClient, RoleServer, ServerHandler, ServiceError,
    model::{
        CallToolRequestParam, CallToolResult, ErrorCode, GetPromptRequestParam, GetPromptResult,
        InitializeRequestParam, ListPromptsResult, ListResourceTemplatesResult,
        ListResourcesResult, ListToolsResult, PaginatedRequestParam, ReadResourceRequestParam,
        ReadResourceResult, ServerCapabilities, ServerInfo, SubscribeRequestParam,
        UnsubscribeRequestParam,
    },
    service::{RequestContext, RunningService},
    tool,
};

fn map_error(e: ServiceError) -> McpError {
    match e {
        ServiceError::McpError(e) => e,
        e => McpError::new(
            ErrorCode::INTERNAL_ERROR,
            "unhandled error",
            Some(e.to_string().into()),
        ),
    }
}

#[derive(Debug)]
pub struct McpProxyServer {
    mcp_proxy_client: RunningService<RoleClient, InitializeRequestParam>,
}

#[tool(tool_box)]
impl McpProxyServer {
    pub fn new(mcp_proxy_client: RunningService<RoleClient, InitializeRequestParam>) -> Self {
        Self { mcp_proxy_client }
    }
}

impl ServerHandler for McpProxyServer {
    async fn get_prompt(
        &self,
        request: GetPromptRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, McpError> {
        self.mcp_proxy_client
            .get_prompt(request)
            .await
            .map_err(map_error)
    }

    async fn list_prompts(
        &self,
        request: Option<PaginatedRequestParam>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, McpError> {
        self.mcp_proxy_client
            .list_prompts(request)
            .await
            .map_err(map_error)
    }

    async fn list_resources(
        &self,
        request: Option<PaginatedRequestParam>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        self.mcp_proxy_client
            .list_resources(request)
            .await
            .map_err(map_error)
    }

    async fn list_resource_templates(
        &self,
        request: Option<PaginatedRequestParam>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        self.mcp_proxy_client
            .list_resource_templates(request)
            .await
            .map_err(map_error)
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParam,
        _: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        self.mcp_proxy_client
            .read_resource(request)
            .await
            .map_err(map_error)
    }

    async fn subscribe(
        &self,
        request: SubscribeRequestParam,
        _: RequestContext<RoleServer>,
    ) -> Result<(), McpError> {
        self.mcp_proxy_client
            .subscribe(request)
            .await
            .map_err(map_error)
    }

    async fn unsubscribe(
        &self,
        request: UnsubscribeRequestParam,
        _: RequestContext<RoleServer>,
    ) -> Result<(), McpError> {
        self.mcp_proxy_client
            .unsubscribe(request)
            .await
            .map_err(map_error)
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        _: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        self.mcp_proxy_client
            .call_tool(request)
            .await
            .map_err(map_error)
    }

    async fn list_tools(
        &self,
        request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        self.mcp_proxy_client
            .list_tools(request)
            .await
            .map_err(map_error)
    }

    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("Dockmaster MCP Proxy Server".into()),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_tool_list_changed()
                .enable_prompts()
                .enable_resources()
                .build(),
            ..Default::default()
        }
    }
}
