# MPC Dockmaster

A desktop application for managing AI applications and Model Context Protocol (MCP) tools. Built with Tauri, React, and TypeScript.

## Features

- Browse and install AI applications from the AI App Store
- Manage installed applications
- Automatic integration with Claude and other MCP-compatible AI assistants
- Support for Node.js, Python, and Docker-based AI tools

## Prerequisites

Before you begin, ensure you have the following installed:

- [Node.js](https://nodejs.org/) (v16 or later)
- [Rust](https://www.rust-lang.org/tools/install)
- Platform-specific dependencies for Tauri:
  - [Windows requirements](https://tauri.app/v1/guides/getting-started/prerequisites#setting-up-windows)
  - [macOS requirements](https://tauri.app/v1/guides/getting-started/prerequisites#setting-up-macos)
  - [Linux requirements](https://tauri.app/v1/guides/getting-started/prerequisites#setting-up-linux)

## Development

1. Clone the repository
2. Navigate to the project directory:
   ```bash
   cd apps/mcp-dockmaster
   ```
3. Install dependencies:
   ```bash
   npm install
   ```
4. Run the development version:
   ```bash
   npm run tauri dev
   ```

This will start the development server and open the application in a native window.

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/)
- [Tauri VS Code Extension](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode)
- [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## Project Structure

- `src/` - React frontend code
- `src-tauri/` - Rust backend code
- `src-tauri/src/features/` - Backend features including MCP proxy and HTTP server

## License

[License information]

This will generate platform-specific installers in the `src-tauri/target/release` directory.

## Integration with Claude

Claude is automatically connected to MPC Dockmaster. After making changes to your apps and tools, refresh Claude by pressing `⌘+R` (macOS) or using the refresh button in the Claude app.
