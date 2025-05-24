// use std::sync::Arc;

// use log::error;
// use serde_json::{json, Value};

// use crate::{
//     core::mcp_core::MCPCore,
//     core::mcp_core_proxy_ext::McpCoreProxyExt,
//     mcp_server::mcp_tools_service::MCPToolsService,
//     models::types::ToolUninstallRequest,
//     registry::registry_search::{RegistrySearch, SearchError},
// };

// use super::notifications::broadcast_tools_list_changed;

// #[derive(Clone)]
// pub struct MCPDockmasterRouter {
//     mcp_core: MCPCore,
//     server_name: String,
//     tools_service: Arc<MCPToolsService>,
// }

// impl MCPDockmasterRouter {
//     /// Create a new MCP router for the Dockmaster server
//     pub async fn new(mcp_core: MCPCore) -> Self {
//         let tools_service = MCPToolsService::initialize(mcp_core.clone()).await;
//         Self {
//             mcp_core,
//             server_name: "mcp-dockmaster-server".to_string(),
//             tools_service,
//         }
//     }

//     /// Update the tools cache and broadcast a notification
//     pub async fn update_tools_cache(&self, operation: &str) -> Result<(), String> {
//         // Update the tools cache
//         if let Err(e) = self.tools_service.update_cache().await {
//             error!("Failed to update tools cache after {}: {}", operation, e);
//             return Err(e);
//         }

//         // Spawn the broadcast notification as a separate task
//         tokio::spawn(async {
//             tokio::time::sleep(std::time::Duration::from_secs(5)).await;
//             broadcast_tools_list_changed().await;
//         });

//         Ok(())
//     }

// }
