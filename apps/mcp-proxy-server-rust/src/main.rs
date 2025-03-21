use anyhow::Result;
use clap::Parser;
use mcp_server::router::RouterService;
use mcp_server::{ByteTransport, Server};
use std::net::SocketAddr;
use tokio::io::{stdin, stdout};
use tokio::net::TcpListener;
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
    let args = Args::parse();

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

    // Create an instance of our counter router
    let dockmaster_router = server::router::DockmasterRouter::new();
    dockmaster_router.initialize().await;
    let router = RouterService(dockmaster_router);

    // Create the server
    let server = Server::new(router);

    // Handle different transport types
    if let Some(port) = args.sse {
        // SSE transport
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let _listener = TcpListener::bind(&addr).await?;
        tracing::info!("SSE server listening on {}", addr);

        // TODO: Implement SSE transport handling
        todo!("SSE transport not yet implemented");
    }

    let transport = ByteTransport::new(stdin(), stdout());

    tracing::info!("Ready to handle requests...");
    Ok(server.run(transport).await?)
}
