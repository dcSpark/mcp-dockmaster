use std::{collections::HashMap, path::PathBuf, sync::Arc};

use log::{error, info};
use tokio::sync::RwLock;

use crate::core::mcp_core_database_ext::McpCoreDatabaseExt;
use crate::core::mcp_core_proxy_ext::McpCoreProxyExt;
use crate::{database::db_manager::DBManager, registry::tool_registry::ToolRegistry};

use crate::mcp_state::mcp_state::MCPState;
use crate::mcp_state::process_manager::ProcessManager;

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
    /// Manager for SQLite database operations
    pub database_manager: Arc<RwLock<DBManager>>,
    /// Registry containing tool metadata and configurations
    pub tool_registry: Arc<RwLock<ToolRegistry>>,
    /// Central state management for the MCP server
    pub mcp_state: Arc<RwLock<MCPState>>,
}

impl MCPCore {
    /// Creates a new MCPCore instance with the given database path
    ///
    /// # Arguments
    /// * `database_path` - Path to the SQLite database file
    ///
    /// # Returns
    /// A new MCPCore instance with initialized components
    pub fn new(database_path: PathBuf) -> Self {
        info!("Creating new MCPCore instance");
        let db_manager = DBManager::with_path(database_path).unwrap();
        let database_manager = Arc::new(RwLock::new(db_manager.clone()));

        let tool_registry = ToolRegistry::with_db_manager(db_manager.clone());
        let tool_registry_arc = Arc::new(RwLock::new(tool_registry));
        let process_manager_arc = Arc::new(RwLock::new(ProcessManager::new()));
        let server_tools_arc = Arc::new(RwLock::new(HashMap::new()));
        let mcp_state_arc = Arc::new(RwLock::new(MCPState::new(
            tool_registry_arc.clone(),
            process_manager_arc.clone(),
            server_tools_arc.clone(),
        )));
        Self {
            database_manager,
            mcp_state: mcp_state_arc,
            tool_registry: tool_registry_arc,
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
        info!("Starting HTTP server");
        if let Err(e) = crate::http_server::start_http_server(self.clone()).await {
            error!("Failed to start HTTP server: {}", e);
            return Err(InitError::StartHttpServer(e.to_string()));
        }
        info!("Initializing MCP server");
        if let Err(e) = self.init_mcp_server().await {
            error!("Failed to initialize MCP server: {}", e);
            return Err(InitError::InitMcpServer(e.to_string()));
        }
        Ok(())
    }
}
