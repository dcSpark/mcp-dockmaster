#[cfg(test)]
mod tests {
    use mcp_core::{models::types::Tool, DBManager};
    use serial_test::serial;
    use std::env;
    use tempfile::tempdir;

    // Helper function to set up a temporary database for testing
    fn setup_temp_db() -> (DBManager, tempfile::TempDir) {
        // Get the temp directory path from the environment or use a default
        let temp_path = env::var("MCP_DATA_DIR").unwrap_or_else(|_| {
            std::env::temp_dir()
                .join("mcp_test_db")
                .to_string_lossy()
                .into_owned()
        });

        // Remove the directory if it exists
        if std::path::Path::new(&temp_path).exists() {
            std::fs::remove_dir_all(&temp_path).expect("Failed to remove existing temp directory");
        }

        // Create a new temporary directory
        let temp_dir = tempdir().expect("Failed to create temp directory");

        // Override the project directory for testing
        env::set_var("MCP_DATA_DIR", temp_dir.path().to_str().unwrap());

        let db = DBManager::new().expect("Failed to create database");
        (db, temp_dir)
    }

    #[test]
    #[serial]
    fn test_save_and_get_tool() {
        let (db, _temp) = setup_temp_db();

        // Create a sample tool
        let tool = Tool {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            enabled: true,
            tool_type: "test".to_string(),
            entry_point: None,
            configuration: None,
            distribution: None,
            env_configs: None,
        };

        // Save the tool
        db.save_tool("test_tool", &tool)
            .expect("Failed to save tool");

        // Get the tool
        let loaded_tool = db.get_tool("test_tool").expect("Failed to get tool");

        // Verify the loaded data matches the original
        assert_eq!(loaded_tool.name, "test_tool");
        assert_eq!(loaded_tool.description, "A test tool");
        assert!(loaded_tool.enabled);
    }

    #[test]
    #[serial]
    fn test_get_all_tools() {
        // Create a unique database path for this test
        use tempfile::tempdir;

        let temp_dir = tempdir().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test_get_all_tools.db");

        // Initialize database with custom path
        let db = DBManager::with_path(db_path).expect("Failed to create database");

        // Create sample tools
        let tool1 = Tool {
            name: "tool1".to_string(),
            description: "Tool 1".to_string(),
            enabled: true,
            tool_type: "test".to_string(),
            entry_point: None,
            configuration: None,
            distribution: None,
            env_configs: None,
        };

        let tool2 = Tool {
            name: "tool2".to_string(),
            description: "Tool 2".to_string(),
            enabled: false,
            tool_type: "test".to_string(),
            entry_point: None,
            configuration: None,
            distribution: None,
            env_configs: None,
        };

        // Save the tools
        db.save_tool("tool1", &tool1).expect("Failed to save tool1");
        db.save_tool("tool2", &tool2).expect("Failed to save tool2");

        // Get all tools
        let tools = db.get_all_tools().expect("Failed to get all tools");

        // Verify the loaded data
        assert_eq!(tools.len(), 2);
        assert_eq!(tools.get("tool1").unwrap().name, "tool1");
        assert_eq!(tools.get("tool2").unwrap().name, "tool2");
    }

    #[test]
    #[serial]
    fn test_clear_database() {
        let (mut db, _temp) = setup_temp_db();

        // Create and save a sample tool
        let tool = Tool {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            enabled: true,
            tool_type: "test".to_string(),
            entry_point: None,
            configuration: None,
            distribution: None,
            env_configs: None,
        };

        db.save_tool("test_tool", &tool)
            .expect("Failed to save tool");

        // Clear the database
        db.clear_database().expect("Failed to clear database");

        // Verify the database is empty
        let tools = db.get_all_tools().expect("Failed to get all tools");
        assert!(tools.is_empty());
    }
}
