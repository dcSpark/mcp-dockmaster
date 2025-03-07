# MCP Core Library

Core functionality for the MCP Dockmaster application written in Rust.

## Overview

This library provides essential functionality for the MCP Dockmaster application, including:

- MCP Proxy system for managing tools and services
- HTTP server for API endpoints
- Database management for persistent storage

## Usage

The library is designed to be used by the Tauri-based MCP Dockmaster application. Import the required modules:

```rust
use mcp_core::http_server::start_http_server;
use mcp_core::mcp_proxy::{MCPState, ToolRegistry};
use mcp_core::database::DatabaseManager;
```

## Development

### Building

Build the library with Cargo:

```bash
nx build mcp-core
# Or
cd libs/mcp-core
cargo build
```

### Testing

Run tests:

```bash
nx test mcp-core
# Or
cd libs/mcp-core
cargo test
```

### Linting

Run linters:

```bash
nx lint mcp-core
# Or
cd libs/mcp-core
cargo fmt --check
cargo clippy
``` 

### Tests

You can run the tests like this:

```bash
cargo test -- --test-threads=1
```