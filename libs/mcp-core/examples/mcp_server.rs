use std::sync::Arc;
use log::info;

// Import from the local crate using the correct crate name
use mcp_dockmaster_core::mcp_server::ClientManager;
use mcp_dockmaster_core::mcp_server::start_mcp_server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging with env_logger
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_secs()
        .init();
    
    // Create a default client manager
    let client_manager = Arc::new(ClientManager {});
    
    // Start the MCP server
    start_mcp_server(client_manager).await?;
    
    Ok(())
} 