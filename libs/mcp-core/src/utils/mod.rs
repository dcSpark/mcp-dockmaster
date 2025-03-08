use std::{fs, path::PathBuf};

use directories::ProjectDirs;
use log::info;

/// Get the default storage path
pub fn default_storage_path() -> Result<PathBuf, String> {
    let proj_dirs = ProjectDirs::from("com", "mcp", "dockmaster")
        .ok_or_else(|| "Failed to determine project directories".to_string())?;

    let storage_path = proj_dirs.data_dir();

    // Ensure the data directory exists
    if !storage_path.exists() {
        fs::create_dir_all(storage_path)
            .map_err(|e| format!("Failed to create data directory: {}", e))?;
    }

    // Check if the directory is writable
    let test_file = storage_path.join(".write_test");
    match fs::File::create(&test_file) {
        Ok(_) => {
            // Clean up the test file
            let _ = fs::remove_file(&test_file);
        }
        Err(e) => {
            return Err(format!("Data directory is not writable: {}", e));
        }
    }

    info!("default storage path: {:?}", storage_path);
    Ok(storage_path.to_path_buf())
}
