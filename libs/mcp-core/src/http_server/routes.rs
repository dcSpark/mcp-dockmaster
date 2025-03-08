use axum::{
    routing::{get, post},
    Extension, Router,
};
use log::{error, info};
use std::net::SocketAddr;

use crate::core::mcp_core::MCPCore;

use super::handlers::{handle_mcp_request, health_check};

pub async fn start_http_server(mcp_core: MCPCore) -> Result<(), String> {
    let app = Router::new()
        .route("/mcp-proxy", post(handle_mcp_request))
        .route("/health", get(health_check))
        .layer(Extension(mcp_core));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    info!("MCP HTTP server starting on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| format!("Failed to bind to address: {}", e))?;
    tokio::spawn(async move {
        match axum::serve(listener, app).await {
            Ok(_) => info!("MCP HTTP server terminated normally"),
            Err(e) => error!("MCP HTTP server error: {}", e),
        }
    });
    Ok(())
}
