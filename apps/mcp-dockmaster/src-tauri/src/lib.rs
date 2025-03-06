use crate::features::mcp_proxy::{
    check_database_exists_command, clear_database_command, discover_tools, execute_proxy_tool,
    get_all_server_data, list_all_server_tools, list_tools, load_mcp_state_command, register_tool,
    restart_tool_command, save_mcp_state_command, uninstall_tool, update_tool_config,
    update_tool_status,
};
use log::{error, info};
use mcp_core::AppContext;
use std::sync::Arc;
use tauri::{Emitter, Manager, RunEvent};
use tray::create_tray;

mod features;
mod tray;

mod commands {
    use std::{
        process::Command,
        sync::atomic::{AtomicBool, Ordering},
    };

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

fn cleanup_mcp_processes(app_handle: tauri::AppHandle) {
    if let Ok(runtime_handle) = tokio::runtime::Handle::try_current() {
        runtime_handle.spawn(async move {
            if let Some(state) = app_handle.try_state::<AppContext>() {
                let app_context = state.clone();
                app_context.kill_all_processes().await;
            }
        });
    }
}

fn init_services(app_handle: tauri::AppHandle, app_context: Arc<AppContext>) {
    tokio::spawn(async move {
        mcp_core::http_server::start_http_server(app_context.clone()).await;
        mcp_core::mcp_proxy::init_mcp_server(app_context).await;

        // Set the initialization complete flag
        commands::INITIALIZATION_COMPLETE.store(true, std::sync::atomic::Ordering::Relaxed);

        info!("Background initialization of MCP services completed");

        // Emit an event to notify the frontend that initialization is complete
        if let Err(e) = app_handle.emit("mcp-initialization-complete", ()) {
            error!("Failed to emit initialization complete event: {}", e);
        } else {
            info!("Emitted initialization complete event");
        }
    });
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

        if let Ok(builder) =
            tauri::WebviewWindowBuilder::from_config(app_handle, &main_window_config)
        {
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
    let app_context = Arc::new(AppContext::new().expect("Failed to create AppContext"));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(app_context.clone())
        .setup(move |app| {
            create_tray(app.handle())?;

            // Start background initialization after the UI has started
            let app_handle = app.handle().clone();
            init_services(app_handle, app_context.clone());

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

                // Cleanup processes
                cleanup_mcp_processes(app_handle.clone());
            }
            RunEvent::Exit { .. } => {
                // Cleanup processes
                cleanup_mcp_processes(app_handle.clone());
                std::process::exit(0);
            }
            #[cfg(target_os = "macos")]
            RunEvent::Reopen { .. } => handle_window_reopen(app_handle),
            _ => {}
        });
}
