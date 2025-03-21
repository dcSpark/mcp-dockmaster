use std::path::PathBuf;
use log::{info, error};

// Import MCP Core components
use mcp_core::core::mcp_core::MCPCore;
use mcp_core::core::mcp_core_database_ext::McpCoreDatabaseExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_secs()
        .init();

    info!("Starting MCP server example with SSE transport");

    // Create temporary paths for database and server binary
    let db_path = PathBuf::from("./mcp_server_example.db");
    let proxy_server_binary_path = PathBuf::from("./proxy_server");
    
    // Create MCPCore instance
    let mcp_core = MCPCore::new_with_port(db_path, proxy_server_binary_path, 3333, "mcp_server_example".to_string());

    // Apply database migrations
    // (This could also be handled by init() but doing it separately for clarity)
    info!("Applying database migrations...");
    if let Err(e) = mcp_core.apply_database_migrations().await {
        error!("Failed to apply database migrations: {}", e);
        return Err(e.into());
    }
    
    // Initialize everything with one call
    // This will:
    // 1. Update the registry cache
    // 2. Start the HTTP server
    // 3. Initialize the MCP server with SSE transport
    // 4. Start any enabled tool processes
    info!("Initializing MCPCore...");
    if let Err(e) = mcp_core.init().await {
        error!("Failed to initialize MCPCore: {:?}", e);
        return Err(format!("MCPCore initialization error: {:?}", e).into());
    }
    
    info!("MCP server initialization complete! Server is running on http://127.0.0.1:3333");
    info!("SSE endpoint: http://127.0.0.1:3333/mcp/sse");
    info!("Press Ctrl+C to stop the server");
    
    // Keep the main thread alive
    tokio::signal::ctrl_c().await?;
    info!("Shutting down MCP server...");
    
    Ok(())
}
