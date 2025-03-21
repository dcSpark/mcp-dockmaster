use anyhow::Result;
use clap::Parser;
use mcp_server::router::RouterService;
use mcp_server::{ByteTransport, Server};
use tokio::io::{stdin, stdout};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{self, EnvFilter};

pub mod server;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Use SSE transport with specified port
    #[arg(long)]
    sse: Option<u16>,
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

    // Create transport
    let transport = ByteTransport::new(stdin(), stdout());

    // Create an instance of our router
    let mut dockmaster_router = server::router::DockmasterRouter::new();
    dockmaster_router.initialize().await;
    let router = RouterService(dockmaster_router);

    // Create the server
    let server = Server::new(router);

    tracing::info!("Ready to handle requests...");
    Ok(server.run(transport).await?)
}
