use log::{error, info};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::AppHandle;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Failed to initialize database: {0}")]
    InitializationError(String),

    #[error("Failed to execute query: {0}")]
    QueryError(String),

    #[error("Failed to get connection from pool: {0}")]
    ConnectionError(String),

    #[error("Failed to serialize/deserialize JSON: {0}")]
    SerializationError(String),
}

pub type DbResult<T> = std::result::Result<T, DatabaseError>;

/// Database connection pool
pub struct Database {
    pool: Pool<SqliteConnectionManager>,
}

impl Database {
    /// Initialize the database with a specified data directory
    pub fn initialize(data_dir: PathBuf) -> DbResult<Self> {
        // Create the directory if it doesn't exist
        std::fs::create_dir_all(&data_dir)
            .map_err(|e| DatabaseError::InitializationError(format!("Failed to create data directory: {}", e)))?;
        
        // Create the database file path
        let db_path = data_dir.join("mcp_dockmaster.db");
        info!("Database path: {:?}", db_path);
        
        // Create the connection manager
        let manager = SqliteConnectionManager::file(db_path);
        
        // Create the connection pool
        let pool = Pool::new(manager)
            .map_err(|e| DatabaseError::InitializationError(format!("Failed to create connection pool: {}", e)))?;
        
        // Initialize the database schema
        let db = Self { pool };
        db.initialize_schema()?;
        
        Ok(db)
    }
    
    /// Initialize the database schema
    fn initialize_schema(&self) -> DbResult<()> {
        let conn = self.pool.get()
            .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;
        
        // Create the tools table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS tools (
                id TEXT PRIMARY KEY,
                data TEXT NOT NULL
            )",
            [],
        )
        .map_err(|e| DatabaseError::QueryError(format!("Failed to create tools table: {}", e)))?;
        
        // Create the server_tools table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS server_tools (
                server_id TEXT NOT NULL,
                tool_data TEXT NOT NULL,
                PRIMARY KEY (server_id)
            )",
            [],
        )
        .map_err(|e| DatabaseError::QueryError(format!("Failed to create server_tools table: {}", e)))?;
        
        info!("Database schema initialized successfully");
        Ok(())
    }
    
    /// Save a tool to the database
    pub fn save_tool(&self, id: &str, data: &Value) -> DbResult<()> {
        let conn = self.pool.get()
            .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;
        
        let data_str = serde_json::to_string(data)
            .map_err(|e| DatabaseError::SerializationError(e.to_string()))?;
        
        conn.execute(
            "INSERT OR REPLACE INTO tools (id, data) VALUES (?, ?)",
            params![id, data_str],
        )
        .map_err(|e| DatabaseError::QueryError(format!("Failed to save tool: {}", e)))?;
        
        Ok(())
    }
    
    /// Load all tools from the database
    pub fn load_tools(&self) -> DbResult<HashMap<String, Value>> {
        let conn = self.pool.get()
            .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;
        
        let mut stmt = conn.prepare("SELECT id, data FROM tools")
            .map_err(|e| DatabaseError::QueryError(format!("Failed to prepare statement: {}", e)))?;
        
        let rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let data_str: String = row.get(1)?;
            Ok((id, data_str))
        })
        .map_err(|e| DatabaseError::QueryError(format!("Failed to query tools: {}", e)))?;
        
        let mut tools = HashMap::new();
        for row_result in rows {
            let (id, data_str) = row_result
                .map_err(|e| DatabaseError::QueryError(format!("Failed to read row: {}", e)))?;
            
            let data: Value = serde_json::from_str(&data_str)
                .map_err(|e| DatabaseError::SerializationError(format!("Failed to parse tool data: {}", e)))?;
            
            tools.insert(id, data);
        }
        
        Ok(tools)
    }
    
    /// Save server tools to the database
    pub fn save_server_tools(&self, server_id: &str, tools: &[Value]) -> DbResult<()> {
        let conn = self.pool.get()
            .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;
        
        let tools_str = serde_json::to_string(tools)
            .map_err(|e| DatabaseError::SerializationError(e.to_string()))?;
        
        conn.execute(
            "INSERT OR REPLACE INTO server_tools (server_id, tool_data) VALUES (?, ?)",
            params![server_id, tools_str],
        )
        .map_err(|e| DatabaseError::QueryError(format!("Failed to save server tools: {}", e)))?;
        
        Ok(())
    }
    
    /// Load all server tools from the database
    pub fn load_server_tools(&self) -> DbResult<HashMap<String, Vec<Value>>> {
        let conn = self.pool.get()
            .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;
        
        let mut stmt = conn.prepare("SELECT server_id, tool_data FROM server_tools")
            .map_err(|e| DatabaseError::QueryError(format!("Failed to prepare statement: {}", e)))?;
        
        let rows = stmt.query_map([], |row| {
            let server_id: String = row.get(0)?;
            let tools_str: String = row.get(1)?;
            Ok((server_id, tools_str))
        })
        .map_err(|e| DatabaseError::QueryError(format!("Failed to query server tools: {}", e)))?;
        
        let mut server_tools = HashMap::new();
        for row_result in rows {
            let (server_id, tools_str) = row_result
                .map_err(|e| DatabaseError::QueryError(format!("Failed to read row: {}", e)))?;
            
            let tools: Vec<Value> = serde_json::from_str(&tools_str)
                .map_err(|e| DatabaseError::SerializationError(format!("Failed to parse server tools data: {}", e)))?;
            
            server_tools.insert(server_id, tools);
        }
        
        Ok(server_tools)
    }
    
    /// Delete a tool from the database
    pub fn delete_tool(&self, id: &str) -> DbResult<()> {
        let conn = self.pool.get()
            .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;
        
        conn.execute(
            "DELETE FROM tools WHERE id = ?",
            params![id],
        )
        .map_err(|e| DatabaseError::QueryError(format!("Failed to delete tool: {}", e)))?;
        
        Ok(())
    }
    
    /// Delete server tools from the database
    pub fn delete_server_tools(&self, server_id: &str) -> DbResult<()> {
        let conn = self.pool.get()
            .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;
        
        conn.execute(
            "DELETE FROM server_tools WHERE server_id = ?",
            params![server_id],
        )
        .map_err(|e| DatabaseError::QueryError(format!("Failed to delete server tools: {}", e)))?;
        
        Ok(())
    }
}
