use std::path::PathBuf;
use serde_json::json;
use crate::features::database::Database;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_database_initialization() {
        // Create a temporary directory for testing
        let temp_dir = std::env::temp_dir().join("mcp_dockmaster_test");
        std::fs::create_dir_all(&temp_dir).expect("Failed to create temp directory");
        
        // Initialize the database
        let _db = Database::initialize(temp_dir.clone()).expect("Failed to initialize database");
        
        // Verify the database file was created
        let db_path = temp_dir.join("mcp_dockmaster.db");
        assert!(db_path.exists(), "Database file was not created");
        
        // Clean up
        std::fs::remove_dir_all(temp_dir).expect("Failed to clean up temp directory");
    }

    #[test]
    fn test_save_and_load_tool() {
        // Create a temporary directory for testing
        let temp_dir = std::env::temp_dir().join("mcp_dockmaster_test_tools");
        std::fs::create_dir_all(&temp_dir).expect("Failed to create temp directory");
        
        // Initialize the database
        let db = Database::initialize(temp_dir.clone()).expect("Failed to initialize database");
        
        // Create a test tool
        let tool_id = "test-tool";
        let tool_data = json!({
            "name": "Test Tool",
            "description": "A tool for testing",
            "version": "1.0.0",
            "enabled": true
        });
        
        // Save the tool
        db.save_tool(tool_id, &tool_data).expect("Failed to save tool");
        
        // Load all tools
        let tools = db.load_tools().expect("Failed to load tools");
        
        // Verify the tool was saved and loaded correctly
        assert!(tools.contains_key(tool_id), "Tool was not found in loaded tools");
        assert_eq!(tools.get(tool_id).unwrap(), &tool_data, "Tool data does not match");
        
        // Clean up
        std::fs::remove_dir_all(temp_dir).expect("Failed to clean up temp directory");
    }

    #[test]
    fn test_save_and_load_server_tools() {
        // Create a temporary directory for testing
        let temp_dir = std::env::temp_dir().join("mcp_dockmaster_test_server_tools");
        std::fs::create_dir_all(&temp_dir).expect("Failed to create temp directory");
        
        // Initialize the database
        let db = Database::initialize(temp_dir.clone()).expect("Failed to initialize database");
        
        // Create test server tools
        let server_id = "test-server";
        let server_tools = vec![
            json!({
                "id": "tool1",
                "name": "Tool 1",
                "description": "First test tool"
            }),
            json!({
                "id": "tool2",
                "name": "Tool 2",
                "description": "Second test tool"
            })
        ];
        
        // Save the server tools
        db.save_server_tools(server_id, &server_tools).expect("Failed to save server tools");
        
        // Load all server tools
        let all_server_tools = db.load_server_tools().expect("Failed to load server tools");
        
        // Verify the server tools were saved and loaded correctly
        assert!(all_server_tools.contains_key(server_id), "Server tools were not found in loaded server tools");
        assert_eq!(all_server_tools.get(server_id).unwrap(), &server_tools, "Server tools data does not match");
        
        // Clean up
        std::fs::remove_dir_all(temp_dir).expect("Failed to clean up temp directory");
    }

    #[test]
    fn test_delete_tool() {
        // Create a temporary directory for testing
        let temp_dir = std::env::temp_dir().join("mcp_dockmaster_test_delete_tool");
        std::fs::create_dir_all(&temp_dir).expect("Failed to create temp directory");
        
        // Initialize the database
        let db = Database::initialize(temp_dir.clone()).expect("Failed to initialize database");
        
        // Create a test tool
        let tool_id = "test-tool-to-delete";
        let tool_data = json!({
            "name": "Test Tool To Delete",
            "description": "A tool for testing deletion",
            "version": "1.0.0",
            "enabled": true
        });
        
        // Save the tool
        db.save_tool(tool_id, &tool_data).expect("Failed to save tool");
        
        // Verify the tool was saved
        let tools_before = db.load_tools().expect("Failed to load tools");
        assert!(tools_before.contains_key(tool_id), "Tool was not found in loaded tools before deletion");
        
        // Delete the tool
        db.delete_tool(tool_id).expect("Failed to delete tool");
        
        // Verify the tool was deleted
        let tools_after = db.load_tools().expect("Failed to load tools");
        assert!(!tools_after.contains_key(tool_id), "Tool was still found in loaded tools after deletion");
        
        // Clean up
        std::fs::remove_dir_all(temp_dir).expect("Failed to clean up temp directory");
    }

    #[test]
    fn test_delete_server_tools() {
        // Create a temporary directory for testing
        let temp_dir = std::env::temp_dir().join("mcp_dockmaster_test_delete_server_tools");
        std::fs::create_dir_all(&temp_dir).expect("Failed to create temp directory");
        
        // Initialize the database
        let db = Database::initialize(temp_dir.clone()).expect("Failed to initialize database");
        
        // Create test server tools
        let server_id = "test-server-to-delete";
        let server_tools = vec![
            json!({
                "id": "tool1",
                "name": "Tool 1",
                "description": "First test tool"
            }),
            json!({
                "id": "tool2",
                "name": "Tool 2",
                "description": "Second test tool"
            })
        ];
        
        // Save the server tools
        db.save_server_tools(server_id, &server_tools).expect("Failed to save server tools");
        
        // Verify the server tools were saved
        let server_tools_before = db.load_server_tools().expect("Failed to load server tools");
        assert!(server_tools_before.contains_key(server_id), "Server tools were not found before deletion");
        
        // Delete the server tools
        db.delete_server_tools(server_id).expect("Failed to delete server tools");
        
        // Verify the server tools were deleted
        let server_tools_after = db.load_server_tools().expect("Failed to load server tools");
        assert!(!server_tools_after.contains_key(server_id), "Server tools were still found after deletion");
        
        // Clean up
        std::fs::remove_dir_all(temp_dir).expect("Failed to clean up temp directory");
    }
}
