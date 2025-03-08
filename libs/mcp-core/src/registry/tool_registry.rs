use std::collections::HashMap;

use crate::{database::db_manager::DBManager, models::types::Tool};

/// ToolRegistry: database logic only
///
/// This module is responsible for reading/writing Tool objects in the database.
/// It has no knowledge about processes or server tools in memory.
pub struct ToolRegistry {
    db: DBManager,
}

impl ToolRegistry {
    pub fn new() -> Result<Self, String> {
        let db = DBManager::new()?;
        Ok(Self { db })
    }

    /// Create a new ToolRegistry with a custom DBManager
    pub fn with_db_manager(db: DBManager) -> Self {
        Self { db }
    }

    /// Get a tool by ID
    pub fn get_tool(&self, tool_id: &str) -> Result<Tool, String> {
        self.db.get_tool(tool_id)
    }

    /// Get all tools
    pub fn get_all_tools(&self) -> Result<HashMap<String, Tool>, String> {
        self.db.get_all_tools()
    }

    /// Save or update a tool
    pub fn save_tool(&self, tool_id: &str, tool: &Tool) -> Result<(), String> {
        self.db.save_tool(tool_id, tool)
    }

    /// Delete a tool
    pub fn delete_tool(&self, tool_id: &str) -> Result<(), String> {
        self.db.delete_tool(tool_id)
    }
}
