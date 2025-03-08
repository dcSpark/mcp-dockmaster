use super::mcp_core::MCPCore;

pub trait McpCoreDatabaseExt {
    fn check_database_exists(
        &self,
    ) -> impl std::future::Future<Output = Result<bool, String>> + Send;
    fn apply_database_migrations(
        &self,
    ) -> impl std::future::Future<Output = Result<(), String>> + Send;
    fn clear_database(&self) -> impl std::future::Future<Output = Result<(), String>> + Send;
}

impl McpCoreDatabaseExt for MCPCore {
    /// Check if the database exists and has data
    async fn check_database_exists(&self) -> Result<bool, String> {
        self.database_manager.read().await.check_exists()
    }
    async fn apply_database_migrations(&self) -> Result<(), String> {
        self.database_manager.write().await.apply_migrations()
    }
    /// Clear all data from the database
    async fn clear_database(&self) -> Result<(), String> {
        match self.database_manager.write().await.clear_database() {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to clear database: {}", e)),
        }
    }
}
