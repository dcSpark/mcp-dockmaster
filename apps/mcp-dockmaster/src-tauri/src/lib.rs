use std::sync::Arc;
use tauri::{Manager, RunEvent, Emitter, State};
use tokio::sync::RwLock;
use log::{info, error};
use mcp_core::features::http_server::start_http_server;
use mcp_core::features::mcp_proxy::{MCPState, ToolRegistry};
use tray::create_tray;

mod features;
mod tray;

// Import the wrapper functions
use crate::features::mcp_wrapper::{
    register_tool, list_tools, list_all_server_tools, discover_tools, execute_tool, 
    execute_proxy_tool, update_tool_status, update_tool_config, uninstall_tool, 
    get_all_server_data, save_mcp_state_command, load_mcp_state_command, 
    check_database_exists_command, clear_database_command, restart_tool_command
};

mod commands {
    use std::{process::Command, sync::atomic::{AtomicBool, Ordering}};
    
    // Global flag to track initialization status
    pub static INITIALIZATION_COMPLETE: AtomicBool = AtomicBool::new(false);
  
    #[tauri::command]
    pub async fn check_node_installed() -> Result<bool, String> {
        match Command::new("node").arg("--version").output() {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
    
    #[tauri::command]
    pub async fn check_uv_installed() -> Result<bool, String> {
        match Command::new("uv").arg("--version").output() {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
    
    #[tauri::command]
    pub async fn check_docker_installed() -> Result<bool, String> {
        match Command::new("docker").arg("--version").output() {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
    
    #[tauri::command]
    pub async fn check_initialization_complete() -> Result<bool, String> {
        Ok(INITIALIZATION_COMPLETE.load(Ordering::Relaxed))
    }
}

fn cleanup_mcp_processes(app_handle: &tauri::AppHandle) {
    if let Some(state) = app_handle.try_state::<MCPState>() {
        let tool_registry = state.tool_registry.clone();
        
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                let mut registry = tool_registry.write().await;
                registry.kill_all_processes().await;
            });
        }
    }
}

async fn init_mcp_services() -> MCPState {
    // Initialize MCP state from database
    let mcp_state = MCPState::default();
    let http_state = Arc::new(RwLock::new(mcp_state.clone()));
    
    info!("Starting MCP HTTP server...");
    start_http_server(http_state).await;
    info!("MCP HTTP server started");
    
    mcp_state
}

/// Initialize MCP services in the background after the UI has started
async fn init_mcp_services_background(mcp_state: MCPState) {
    info!("Starting background initialization of MCP services...");
    
    // Use the existing initialize_mcp_state function which handles loading from DB and restarting tools
    let initialized_state = ToolRegistry::init_mcp_state().await;
    
    // Copy the initialized data to our existing state
    {
        let initialized_registry = initialized_state.tool_registry.read().await;
        let mut current_registry = mcp_state.tool_registry.write().await;
        
        // Copy the tools and server_tools data
        current_registry.tools = initialized_registry.tools.clone();
        current_registry.server_tools = initialized_registry.server_tools.clone();
        
        // Mark processes as running in the tool data if they are running in the initialized state
        for (tool_id, process_opt) in &initialized_registry.processes {
            let is_running = process_opt.is_some();
            if is_running {
                // Update the tool data to mark it as running
                if let Some(tool_data) = current_registry.tools.get_mut(tool_id) {
                    if let Some(obj) = tool_data.as_object_mut() {
                        // Set a flag in the tool data to indicate it's running
                        obj.insert("process_running".to_string(), serde_json::json!(true));
                    }
                }
                
                // Also update the processes map with None to indicate it's running
                current_registry.processes.insert(tool_id.clone(), None);
            }
        }
    }
    
    // Set the initialization complete flag
    commands::INITIALIZATION_COMPLETE.store(true, std::sync::atomic::Ordering::Relaxed);
    
    info!("Background initialization of MCP services completed");
}

#[cfg(target_os = "macos")]
fn handle_window_reopen(app_handle: &tauri::AppHandle) {
    let main_window_label = "main";
    
    if let Some(window) = app_handle.get_webview_window(main_window_label) {
        window.show().unwrap();
        window.center().unwrap();
        let _ = window.set_focus();
    } else {
        let main_window_config = app_handle
            .config()
            .app
            .windows
            .iter()
            .find(|w| w.label == main_window_label)
            .unwrap()
            .clone();
            
        if let Ok(builder) = tauri::WebviewWindowBuilder::from_config(app_handle, &main_window_config) {
            if let Err(e) = builder.build() {
                eprintln!("Failed to build main window: {}", e);
            }
        } else {
            eprintln!("Failed to create WebviewWindowBuilder from config");
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() {
    let mcp_state = init_mcp_services().await;
    let mcp_state_clone = mcp_state.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(mcp_state.clone())
        .setup(|app| {
            create_tray(app.handle())?;
            
            // Start background initialization after the UI has started
            let app_handle = app.handle().clone();
            tokio::spawn(async move {
                init_mcp_services_background(mcp_state_clone).await;
                
                // Emit an event to notify the frontend that initialization is complete
                if let Err(e) = app_handle.emit("mcp-initialization-complete", ()) {
                    error!("Failed to emit initialization complete event: {}", e);
                } else {
                    info!("Emitted initialization complete event");
                }
            });
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::check_node_installed,
            commands::check_uv_installed,
            commands::check_docker_installed,
            commands::check_initialization_complete,
            register_tool,
            list_tools,
            list_all_server_tools,
            discover_tools,
            execute_tool,
            execute_proxy_tool,
            update_tool_status,
            update_tool_config,
            restart_tool_command,
            uninstall_tool,
            get_all_server_data,
            save_mcp_state_command,
            load_mcp_state_command,
            check_database_exists_command,
            clear_database_command
        ])
        .build(tauri::generate_context!())
        .expect("Error while running Tauri application")
        .run(move |app_handle, event| match event {
            RunEvent::ExitRequested { api, .. } => {
                // First, prevent exit to handle cleanup
                api.prevent_exit();
                
                // Clone the entire state to avoid lifetime issues
                if let Some(state) = app_handle.try_state::<MCPState>() {
                    // Create a deep clone that's fully owned
                    let state_owned = state.inner().clone();
                    
                    // Spawn a task to save state and then cleanup
                    std::thread::spawn(move || {
                        // Create a new runtime for this thread
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        rt.block_on(async {
                            if let Err(e) = ToolRegistry::save_mcp_state(&state_owned).await {
                                log::error!("Failed to save MCP state: {}", e);
                            } else {
                                log::info!("MCP state saved successfully");
                            }
                        });
                    });
                }
                
                // Cleanup processes
                cleanup_mcp_processes(app_handle);
            }
            RunEvent::Exit { .. } => {
                // Clone the entire state to avoid lifetime issues
                if let Some(state) = app_handle.try_state::<MCPState>() {
                    // Create a deep clone that's fully owned
                    let state_owned = state.inner().clone();
                    
                    // Use a separate thread to avoid blocking the main thread
                    std::thread::spawn(move || {
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        rt.block_on(async {
                            if let Err(e) = ToolRegistry::save_mcp_state(&state_owned).await {
                                log::error!("Failed to save MCP state: {}", e);
                            } else {
                                log::info!("MCP state saved successfully");
                            }
                        });
                    });
                }
                
                // Cleanup processes
                cleanup_mcp_processes(app_handle);
                std::process::exit(0);
            }
            #[cfg(target_os = "macos")]
            RunEvent::Reopen { .. } => handle_window_reopen(app_handle),
            _ => {}
        });
}
