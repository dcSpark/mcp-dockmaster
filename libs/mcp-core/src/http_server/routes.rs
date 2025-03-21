use axum::{
    routing::{get, post},
    Extension, Router,
};
use log::{error, info};
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;

use crate::core::mcp_core::MCPCore;
use crate::http_server::handlers::{handle_mcp_request, health_check, sse_handler, sse_post_handler};
use crate::mcp_server::{MCPDockmasterRouter};

pub async fn start_http_server(mcp_core: MCPCore, port: u16) -> Result<(), String> {
    // Create our MCP router that will handle RPC requests
    let mcp_router = MCPDockmasterRouter::new(mcp_core.clone());
    mcp_router.update_tools_cache().await;

    // Set up the HTTP routes
    let app = Router::new()
        .route("/", get(health_check))
        .route("/health", get(health_check))
        .route("/mcp/sse", get(sse_handler).post(sse_post_handler))
        .route("/mcp-proxy", post(handle_mcp_request))
        .route("/mcp", post(handle_mcp_request))
        .layer(Extension(mcp_core.clone()))
        .layer(Extension(mcp_router))
        .layer(CorsLayer::permissive());

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    info!("MCP HTTP server starting on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| format!("Failed to bind to address: {}", e))?;

    // Start the server in a separate task
    tokio::spawn(async move {
        // With axum 0.8.1, we need to create a service
        let service = app.into_make_service();
        if let Err(e) = axum::serve(listener, service).await {
            error!("MCP HTTP server error: {}", e);
        } else {
            info!("MCP HTTP server terminated normally");
        }
    });

    Ok(())
}
