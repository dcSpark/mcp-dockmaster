use crate::features::mcp_proxy::{
    check_database_exists_command, clear_database_command, discover_tools, execute_proxy_tool,
    import_server_from_url, list_all_server_tools, list_servers, register_server,
    restart_server_command, set_tools_hidden, uninstall_server, update_server_config, update_server_status,
};
use features::mcp_proxy::{
    check_claude_installed, check_cursor_installed, get_claude_config, get_cursor_config,
    get_generic_config, install_claude, install_cursor, is_process_running, restart_process,
};
use log::{error, info};
use mcp_core::core::{mcp_core::MCPCore, mcp_core_proxy_ext::McpCoreProxyExt};
use tauri::{utils::platform, Emitter, Manager, RunEvent};
use tray::create_tray;
use updater::{check_for_updates, check_for_updates_command};

mod features;
mod tray;
mod updater;

mod commands {
    use std::sync::atomic::{AtomicBool, Ordering};

    use mcp_core::core::{mcp_core::MCPCore, mcp_core_runtimes_ext::McpCoreRuntimesExt};

    // Global flag to track initialization status
    pub static INITIALIZATION_COMPLETE: AtomicBool = AtomicBool::new(false);

    #[tauri::command]
    pub async fn check_node_installed() -> Result<bool, String> {
        MCPCore::is_nodejs_installed().await
    }

    #[tauri::command]
    pub async fn check_uv_installed() -> Result<bool, String> {
        MCPCore::is_uv_installed().await
    }

    #[tauri::command]
    pub async fn check_docker_installed() -> Result<bool, String> {
        MCPCore::is_docker_installed().await
    }

    #[tauri::command]
    pub async fn check_initialization_complete() -> Result<bool, String> {
        Ok(INITIALIZATION_COMPLETE.load(Ordering::Relaxed))
    }
}
fn cleanup_mcp_processes(app_handle: &tauri::AppHandle) {
    let mcp_core = app_handle.state::<MCPCore>();
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        let mcp_core = mcp_core.inner().clone();
        handle.spawn(async move {
            let result = mcp_core.kill_all_processes().await;
            if let Err(e) = result {
                error!("Failed to kill all MCP processes: {}", e);
            }
        });
    }
}

fn init_services(app_handle: tauri::AppHandle) {
    tokio::spawn(async move {
        let mcp_core = app_handle.state::<MCPCore>();
        let result = mcp_core.init().await;
        if let Err(e) = result {
            error!("Failed to initialize MCP services: {:?}", e);
        }

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

fn init_mcp_core(app_handle: &tauri::AppHandle) -> Result<(), String> {
    let proxy_server_sidecar_name = "mcp-proxy-server";
    let proxy_server_sidecar_path = match platform::current_exe()
        .map_err(|_| "failed to get current exe")?
        .parent()
    {
        #[cfg(windows)]
        Some(exe_dir) => Ok(exe_dir
            .join(proxy_server_sidecar_name)
            .with_extension("exe")),
        #[cfg(not(windows))]
        Some(exe_dir) => Ok(exe_dir.join(proxy_server_sidecar_name)),
        None => Err("failed to get proxy server sidecar path".to_string()),
    }?;

    if !proxy_server_sidecar_path.exists() {
        let error_message = format!(
            "proxy server sidecar binary not found in path {}",
            proxy_server_sidecar_path.display()
        );
        return Err(error_message);
    }
    info!(
        "Proxy server sidecar path: {}",
        proxy_server_sidecar_path.display()
    );

    let database_path = app_handle
        .path()
        .app_data_dir()
        .map_err(|_| "failed to get app data dir")?
        .join("mcp_dockmaster.db");

    info!("database path: {}", database_path.display());

    let mcp_core = MCPCore::new(database_path, proxy_server_sidecar_path.to_path_buf());
    app_handle.manage(mcp_core.clone());
    Ok(())
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
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {}))
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            create_tray(app.handle())?;

            // Check for updates in the background when the app is opened
            let app_handle_clone = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let _ = check_for_updates(&app_handle_clone, false).await;
            });

            init_mcp_core(app.handle())?;

            // Start background initialization after the UI has started
            let app_handle = app.handle().clone();
            init_services(app_handle);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::check_node_installed,
            commands::check_uv_installed,
            commands::check_docker_installed,
            commands::check_initialization_complete,
            register_server,
            list_servers,
            list_all_server_tools,
            discover_tools,
            execute_proxy_tool,
            update_server_status,
            update_server_config,
            restart_server_command,
            uninstall_server,
            check_database_exists_command,
            clear_database_command,
            check_claude_installed,
            check_cursor_installed,
            install_claude,
            install_cursor,
            get_claude_config,
            get_cursor_config,
            get_generic_config,
            import_server_from_url,
            restart_process,
            is_process_running,
            check_for_updates_command,
            set_tools_hidden
        ])
        .build(tauri::generate_context!())
        .expect("Error while running Tauri application")
        .run(move |app_handle, event| match event {
            RunEvent::ExitRequested { api, .. } => {
                // First, prevent exit to handle cleanup
                api.prevent_exit();

                // Cleanup processes
                cleanup_mcp_processes(app_handle);
            }
            RunEvent::Exit { .. } => {
                // Cleanup processes
                cleanup_mcp_processes(app_handle);
                std::process::exit(0);
            }
            #[cfg(target_os = "macos")]
            RunEvent::Reopen { .. } => handle_window_reopen(app_handle),
            _ => {}
        });
}
