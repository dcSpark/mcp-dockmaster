use anyhow::Result;
use clap::Parser;
use rmcp::{ServiceExt, transport::stdio};
use server::mcp_proxy_client::get_mcp_client;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{self, EnvFilter};

pub mod server;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Use SSE transport with specified port
    #[arg(long)]
    see_target_address: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Set up file appender for logging
    let temp_path = std::env::temp_dir().join("mcp-server-logs");
    let file_appender = RollingFileAppender::new(Rotation::DAILY, temp_path, "proxy-server.log");

    // Initialize the tracing subscriber with file and stdout logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .with_writer(file_appender)
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    tracing::info!("Starting MCP Dockmaster Proxy Server");

    let args = Args::parse();
    let sse_address = args
        .see_target_address
        .unwrap_or("http://localhost:11011/mcp/sse".to_string());
    let mcp_proxy_client = get_mcp_client(&sse_address).await.unwrap();

    tracing::info!("Creating stdio transport...");
    let transport = stdio();

    tracing::info!("Creating MCP server...");
    let mcp_proxy_server = server::mcp_proxy_server::McpProxyServer::new(mcp_proxy_client)
        .serve(transport)
        .await?;

    mcp_proxy_server.waiting().await?;
    Ok(())
}
