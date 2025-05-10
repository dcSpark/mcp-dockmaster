use anyhow::Result;
use rmcp::{
    RoleClient, ServiceExt,
    model::{ClientCapabilities, ClientInfo, Implementation, InitializeRequestParam},
    service::RunningService,
    transport::SseTransport,
};

pub async fn get_mcp_client(
    server_url: &str,
) -> Result<RunningService<RoleClient, InitializeRequestParam>> {
    let transport = SseTransport::start(server_url).await?;
    let client_info = ClientInfo {
        protocol_version: Default::default(),
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "dockmaster-mcp-proxy-server-client".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
    };
    let client = client_info.serve(transport).await.inspect_err(|e| {
        tracing::error!("client error: {:?}", e);
    })?;

    if cfg!(debug_assertions) {
        tracing::debug!("MCP client initialized with server URL: {}", server_url);
        tracing::debug!("MCP client peer info: {:?}", client.peer_info());
        tracing::debug!("MCP client tools: {:?}", client.list_tools(None).await?);
        tracing::debug!(
            "MCP client resources: {:?}",
            client.list_resources(None).await?
        );
        tracing::debug!("MCP client prompts: {:?}", client.list_prompts(None).await?);
    }
    Ok(client)
}
