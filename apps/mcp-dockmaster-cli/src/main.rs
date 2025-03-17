use std::path;

use clap::{Parser, Subcommand};
use log::{error, info};
use mcp_core::{
    core::{
        mcp_core::MCPCore, mcp_core_database_ext::McpCoreDatabaseExt,
        mcp_core_proxy_ext::McpCoreProxyExt,
    },
    init_logging,
    utils::default_storage_path,
};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Register a new tool
    Register {
        /// Server name
        #[arg(short, long)]
        name: String,

        /// Server description
        #[arg(short, long)]
        description: String,

        /// Server type (node, python, docker)
        #[arg(short, long)]
        tools_type: String,

        /// Entry point (command or file path)
        #[arg(short, long)]
        entry_point: String,
    },

    /// List registered tools
    List,

    /// Execute a tool
    Execute {
        /// Tool ID
        #[arg(short, long)]
        tool_id: String,

        /// Parameters (JSON string)
        #[arg(short, long)]
        parameters: Option<String>,
    },

    /// Update a tool's status
    Update {
        /// Server ID
        #[arg(short, long)]
        server_id: String,

        /// Enable or disable the tool
        #[arg(short, long)]
        enabled: bool,
    },

    /// Update a tool's configuration
    Config {
        /// Server ID
        #[arg(short, long)]
        server_id: String,

        /// Environment variable (format: KEY=VALUE)
        #[arg(short, long)]
        env: Vec<String>,
    },

    /// Uninstall a tool
    Uninstall {
        /// Tool ID
        #[arg(short, long)]
        tool_id: String,
    },

    /// Restart a tool
    Restart {
        /// Tool ID
        #[arg(short, long)]
        server_id: String,
    },

    /// Clear the database
    Clear,
}

#[tokio::main]
async fn main() {
    // Initialize logging
    init_logging();

    // Parse command line arguments
    let cli = Cli::parse();

    // Initialize MCP state
    let storage_path = match default_storage_path() {
        Ok(path) => path,
        Err(e) => {
            eprintln!("failed to get storage path: {}", e);
            std::process::exit(1);
        }
    };
    let database_path = storage_path.join("mcp_dockmaster.db");

    // TODO: We should implement a way to bundle/download the mcp-proxy-server location so we can pass this param to the library
    let mcp_core = MCPCore::new(database_path, path::absolute("mcp-proxy-server").unwrap());
    mcp_core.init().await.unwrap();

    // Handle commands
    match cli.command {
        Commands::Register { name, .. } => {
            info!("Registering tool: {}", name);

            // We can't directly create ToolRegistrationRequest due to private fields
            // Instead, we'll use a different approach to register tools
            println!("Tool registration is not directly supported through the CLI.");
            println!("Please use the MCP Dockmaster UI to register tools.");
        }
        Commands::List => {
            info!("Listing tools");

            // Get servers and tools data
            let servers = match mcp_core.list_servers().await {
                Ok(servers) => servers,
                Err(e) => {
                    error!("Error listing servers: {}", e);
                    println!("Error listing servers: {}", e);
                    return;
                }
            };

            let tools = match mcp_core.list_all_server_tools().await {
                Ok(tools) => tools,
                Err(e) => {
                    error!("Error listing tools: {}", e);
                    println!("Error listing tools: {}", e);
                    return;
                }
            };

            // Print servers
            println!("Registered Servers:");
            for (i, server) in servers.iter().enumerate() {
                println!("{}. {}", i + 1, server.definition.name);
                println!("   ID: {}", server.id);
                println!("   Type: {}", server.definition.tools_type);
                println!("   Status: {}", server.status);
                println!("   Tool Count: {}", server.tool_count);
                println!();
            }

            // Print tools
            println!("Available Tools:");
            for (i, tool) in tools.iter().enumerate() {
                println!("{}. {}", i + 1, tool.name);
                println!("   ID: {}", tool.id);
                println!("   Server: {}", tool.server_id);
                println!(
                    "   Proxy ID: {}",
                    tool.proxy_id.as_deref().unwrap_or("None")
                );
                println!("   Description: {}", tool.description);
                println!();
            }
        }
        Commands::Execute { tool_id, .. } => {
            info!("Executing tool: {}", tool_id);

            // We can't directly create ToolExecutionRequest due to private fields
            // Instead, we'll use a different approach to execute tools
            println!("Tool execution is not directly supported through the CLI.");
            println!("Please use the MCP Dockmaster UI to execute tools.");
        }
        Commands::Update { server_id, enabled } => {
            info!(
                "Updating server status: {} (enabled={})",
                server_id, enabled
            );

            // We can't directly create ToolUpdateRequest due to private fields
            // Instead, we'll use a different approach to update tool status
            println!("Tool status update is not directly supported through the CLI.");
            println!("Please use the MCP Dockmaster UI to update tool status.");
        }
        Commands::Config { server_id, .. } => {
            info!("Updating server configuration: {}", server_id);

            // We can't directly create ToolConfigUpdateRequest due to private fields
            // Instead, we'll use a different approach to update tool configuration
            println!("Tool configuration update is not directly supported through the CLI.");
            println!("Please use the MCP Dockmaster UI to update tool configuration.");
        }
        Commands::Uninstall { tool_id } => {
            info!("Uninstalling tool: {}", tool_id);

            // We can't directly create ToolUninstallRequest due to private fields
            // Instead, we'll use a different approach to uninstall tools
            println!("Tool uninstallation is not directly supported through the CLI.");
            println!("Please use the MCP Dockmaster UI to uninstall tools.");
        }
        Commands::Restart { server_id } => {
            info!("Restarting server: {}", server_id);

            // Restart the server using the direct function
            match mcp_core.restart_server_command(server_id.clone()).await {
                Ok(_) => {
                    println!("Server restarted successfully");
                }
                Err(e) => {
                    error!("Error restarting server: {}", e);
                    println!("Error restarting server: {}", e);
                }
            }
        }
        Commands::Clear => {
            info!("Clearing database");

            // Clear the database using the direct function
            match mcp_core.clear_database().await {
                Ok(_) => {
                    println!("Database cleared successfully");
                }
                Err(e) => {
                    error!("Error clearing database: {}", e);
                    println!("Error clearing database: {}", e);
                }
            }
        }
    }
}
