use std::{collections::HashMap, path::PathBuf, sync::Arc};

use log::{error, info, warn};
use rmcp::ServiceExt;
use tokio::sync::RwLock;

use crate::core::mcp_core_database_ext::McpCoreDatabaseExt;
use crate::core::mcp_core_proxy_ext::McpCoreProxyExt;
use crate::database::db_manager::DBManager;
use crate::mcp_server::mcp_server::McpServer;
use crate::registry::server_registry::ServerRegistry;

use crate::mcp_state::mcp_state::MCPState;
use rmcp::transport::stdio;

/// Errors that can occur during initialization
#[derive(Debug)]
pub enum InitError {
    /// Error initializing the database
    ApplyMigrations(String),
    /// Error initializing the MCP server
    StartHttpServer(String),
    /// Error initializing the MCP server
    InitMcpServer(String),
}

#[derive(Clone)]
/// Core struct that manages the MCP server state and components
pub struct MCPCore {
    /// Path to the proxy server binary
    pub proxy_server_binary_path: PathBuf,
    /// Manager for SQLite database operations
    pub database_manager: Arc<RwLock<DBManager>>,
    /// Registry containing server metadata and configurations
    pub tool_registry: Arc<RwLock<ServerRegistry>>,
    /// Central state management for the MCP server
    pub mcp_state: Arc<RwLock<MCPState>>,
    /// HTTP server port
    pub port: u16,
    /// App name
    pub app_name: String,
}

impl MCPCore {
    /// Creates a new MCPCore instance with the given database path
    ///
    /// # Arguments
    /// * `database_path` - Path to the SQLite database file
    ///
    /// # Returns
    /// A new MCPCore instance with initialized components
    pub fn new(
        database_path: PathBuf,
        proxy_server_binary_path: PathBuf,
        app_name: String,
    ) -> Self {
        let port = std::env::var("DOCKMASTER_HTTP_SERVER_PORT")
            .unwrap_or_else(|_| "11011".to_string())
            .parse::<u16>()
            .unwrap_or(11011);
        Self::new_with_port(database_path, proxy_server_binary_path, port, app_name)
    }

    /// Creates a new MCPCore instance with the given database path and port
    ///
    /// # Arguments
    /// * `database_path` - Path to the SQLite database file
    /// * `proxy_server_binary_path` - Path to the proxy server binary
    /// * `port` - HTTP server port
    ///
    /// # Returns
    /// A new MCPCore instance with initialized components
    pub fn new_with_port(
        database_path: PathBuf,
        proxy_server_binary_path: PathBuf,
        port: u16,
        app_name: String,
    ) -> Self {
        info!("Creating new MCPCore instance");
        let db_manager = DBManager::with_path(database_path).unwrap();
        let database_manager = Arc::new(RwLock::new(db_manager.clone()));

        let tool_registry = ServerRegistry::with_db_manager(db_manager.clone());
        let tool_registry_arc = Arc::new(RwLock::new(tool_registry));
        let server_tools_arc = Arc::new(RwLock::new(HashMap::new()));
        let mcp_clients_arc = Arc::new(RwLock::new(HashMap::new()));
        let mcp_state_arc = Arc::new(RwLock::new(MCPState::new(
            tool_registry_arc.clone(),
            server_tools_arc.clone(),
            mcp_clients_arc.clone(),
        )));
        Self {
            proxy_server_binary_path,
            database_manager,
            mcp_state: mcp_state_arc,
            tool_registry: tool_registry_arc,
            port,
            app_name,
        }
    }

    /// Initializes the MCP server by starting the HTTP server and background services
    ///
    /// This function starts:
    /// - The HTTP server for handling API requests
    /// - Background services for managing tools and processes
    pub async fn init(&self) -> Result<(), InitError> {
        info!("Initializing MCP server");
        info!("Applying database migrations");
        if let Err(e) = self.apply_database_migrations().await {
            error!("Failed to apply database migrations: {}", e);
            return Err(InitError::ApplyMigrations(e.to_string()));
        }

        // Update registry cache before starting the server
        info!("Updating registry cache before server initialization");
        match crate::registry::registry_cache::RegistryCache::instance()
            .update_registry_cache()
            .await
        {
            Ok(_) => info!("Registry cache successfully updated"),
            Err(e) => warn!("Warning: Failed to update registry cache: {}", e),
        }
        info!("Initializing Background MCP servers");
        if let Err(e) = self.init_mcp_server().await {
            error!("Failed to initialize MCP server: {}", e);
            return Err(InitError::InitMcpServer(e.to_string()));
        }

        info!("Creating MCP server...");
        let transport = stdio();
        let mcp_server = McpServer::new(self.clone())
            .serve(transport)
            .await
            .map_err(|e| InitError::InitMcpServer(e.to_string()))?;
        mcp_server
            .waiting()
            .await
            .map_err(|e| InitError::InitMcpServer(e.to_string()))?;

        Ok(())
    }

    /// Get the current tool visibility state
    pub async fn are_tools_hidden(&self) -> bool {
        let mcp_state = self.mcp_state.read().await;
        mcp_state.are_tools_hidden().await
    }
}
