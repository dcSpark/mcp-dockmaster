# MCP Server

This module provides a server implementation for the Model Context Protocol (MCP) based on the `mcp-server` and `mcp-core` crates from the Rust SDK.

## Features

- JSON-RPC over stdin/stdout
- Tool registration and execution
- Based on the official MCP Rust SDK

## Running the Server

The MCP server example provides a simple implementation that you can run out of the box:

```bash
cargo run --example mcp_server
```

This will start an MCP server that listens on stdin/stdout for MCP protocol messages.

## Server Details

The server uses stdin/stdout for communication with the client. It uses the JSON-RPC protocol for communication and provides the following capabilities:

- Tool execution: Allows clients to call tools implemented by the server
- Tool listing: Allows clients to discover available tools

## Testing

You can test the server using the MCP Inspector in one of two ways:

### Option 1: Run the server and connect the inspector to it

First, build and run your server:
```bash
cargo run --example mcp_server
```

Then in a separate terminal, run the MCP Inspector:
```bash
npx @modelcontextprotocol/inspector
```

The inspector should detect your server running on stdin/stdout.

### Option 2: Use the inspector to run your server directly:

You can also have the inspector run your server directly:
```bash
npx @modelcontextprotocol/inspector cargo run --example mcp_server
```

This will start the inspector, which will run your server and connect to it.

## Running the Server from Anywhere

You can run the MCP server from any directory without changing your current working directory by using the `--manifest-path` flag:

```bash
cargo run --example mcp_server --manifest-path /path/to/mcp-dockmaster/libs/mcp-core/Cargo.toml
```

This approach is particularly useful for:
- Running the server from scripts located in different directories
- Creating shell aliases for quick server startup
- Integrating with other tools that may be in different directories

For convenience, you might want to create a shell alias in your `~/.zshrc` or `~/.bashrc`:

```bash
alias mcp-server='cargo run --example mcp_server --manifest-path /path/to/mcp-dockmaster/libs/mcp-core/Cargo.toml'
```

After adding this and reloading your shell configuration (`source ~/.zshrc`), you can simply type `mcp-server` to start the server from anywhere.

## Example Code

The example server implementation is located at `examples/mcp_server.rs`. It creates a default ClientManager and starts the MCP server:

```rust
use std::sync::Arc;
use log::info;

// Import from the local crate
use mcp_dockmaster_core::mcp_server::ClientManager;
use mcp_dockmaster_core::mcp_server::start_mcp_server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging with env_logger
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_secs()
        .init();
    
    info!("Starting MCP server...");
    
    // Create a default client manager
    let client_manager = Arc::new(ClientManager {});
    
    // Start the MCP server
    start_mcp_server(client_manager).await?;
    
    Ok(())
} 