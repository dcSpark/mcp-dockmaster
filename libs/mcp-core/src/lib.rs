pub mod core;
pub mod database;
pub mod jsonrpc_frame_codec;
pub mod mcp_installers;
pub mod mcp_server;
pub mod mcp_state;
pub mod models;
pub mod registry;
pub mod schema;

// Re-export commonly used types and functions
pub use models::*;
pub mod utils;

// pub use registry::ToolRegistry;

use std::sync::Once;

static INIT: Once = Once::new();

// Initialize logging
pub fn init_logging() {
    INIT.call_once(|| {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
            .format_timestamp_secs()
            .init();

        log::info!("MCP Core library initialized");
    });
}
