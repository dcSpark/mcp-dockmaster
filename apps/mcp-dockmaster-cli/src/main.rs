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
        /// Tool name
        #[arg(short, long)]
        name: String,

        /// Tool description
        #[arg(short, long)]
        description: String,

        /// Tool type (node, python, docker)
        #[arg(short, long)]
        tool_type: String,

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
        /// Tool ID
        #[arg(short, long)]
        tool_id: String,

        /// Enable or disable the tool
        #[arg(short, long)]
        enabled: bool,
    },

    /// Update a tool's configuration
    Config {
        /// Tool ID
        #[arg(short, long)]
        tool_id: String,

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
        tool_id: String,
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
    let mcp_core = MCPCore::new(database_path);
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

            // Get all server data
            match mcp_core.get_all_server_data().await {
                Ok(data) => {
                    // Print servers
                    if let Some(servers) = data.get("servers").and_then(|s| s.as_array()) {
                        println!("Registered Servers:");
                        for (i, server) in servers.iter().enumerate() {
                            println!(
                                "{}. {}",
                                i + 1,
                                server
                                    .get("name")
                                    .and_then(|n| n.as_str())
                                    .unwrap_or("Unknown")
                            );
                            println!(
                                "   ID: {}",
                                server
                                    .get("id")
                                    .and_then(|id| id.as_str())
                                    .unwrap_or("Unknown")
                            );
                            println!(
                                "   Type: {}",
                                server
                                    .get("tool_type")
                                    .and_then(|t| t.as_str())
                                    .unwrap_or("Unknown")
                            );
                            println!(
                                "   Running: {}",
                                server
                                    .get("process_running")
                                    .and_then(|p| p.as_bool())
                                    .unwrap_or(false)
                            );
                            println!(
                                "   Tool Count: {}",
                                server
                                    .get("tool_count")
                                    .and_then(|c| c.as_i64())
                                    .unwrap_or(0)
                            );
                            println!();
                        }
                    }

                    // Print tools
                    if let Some(tools) = data.get("tools").and_then(|t| t.as_array()) {
                        println!("Available Tools:");
                        for (i, tool) in tools.iter().enumerate() {
                            println!(
                                "{}. {}",
                                i + 1,
                                tool.get("name")
                                    .and_then(|n| n.as_str())
                                    .unwrap_or("Unknown")
                            );
                            println!(
                                "   ID: {}",
                                tool.get("id")
                                    .and_then(|id| id.as_str())
                                    .unwrap_or("Unknown")
                            );
                            println!(
                                "   Server: {}",
                                tool.get("server_id")
                                    .and_then(|s| s.as_str())
                                    .unwrap_or("Unknown")
                            );
                            println!(
                                "   Proxy ID: {}",
                                tool.get("proxy_id")
                                    .and_then(|p| p.as_str())
                                    .unwrap_or("Unknown")
                            );
                            println!(
                                "   Description: {}",
                                tool.get("description")
                                    .and_then(|d| d.as_str())
                                    .unwrap_or("")
                            );
                            println!();
                        }
                    }
                }
                Err(e) => {
                    error!("Error listing tools: {}", e);
                    println!("Error listing tools: {}", e);
                }
            }
        }
        Commands::Execute { tool_id, .. } => {
            info!("Executing tool: {}", tool_id);

            // We can't directly create ToolExecutionRequest due to private fields
            // Instead, we'll use a different approach to execute tools
            println!("Tool execution is not directly supported through the CLI.");
            println!("Please use the MCP Dockmaster UI to execute tools.");
        }
        Commands::Update { tool_id, enabled } => {
            info!("Updating tool status: {} (enabled={})", tool_id, enabled);

            // We can't directly create ToolUpdateRequest due to private fields
            // Instead, we'll use a different approach to update tool status
            println!("Tool status update is not directly supported through the CLI.");
            println!("Please use the MCP Dockmaster UI to update tool status.");
        }
        Commands::Config { tool_id, .. } => {
            info!("Updating tool configuration: {}", tool_id);

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
        Commands::Restart { tool_id } => {
            info!("Restarting tool: {}", tool_id);

            // Restart the tool using the direct function
            match mcp_core.restart_tool_command(tool_id.clone()).await {
                Ok(_) => {
                    println!("Tool restarted successfully");
                }
                Err(e) => {
                    error!("Error restarting tool: {}", e);
                    println!("Error restarting tool: {}", e);
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
