use crate::models::types::{ErrorResponse, RegistryTool, RegistryToolsResponse};
use crate::registry::registry_cache::RegistryCache;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Registry service for working with the tool registry
pub struct RegistryService;

impl RegistryService {
    /// Fetch and return the tool registry
    pub async fn fetch_registry() -> Result<RegistryToolsResponse, ErrorResponse> {
        RegistryCache::instance().get_registry_tools().await
    }

    /// Fetch and return the tool registry (sync version)
    pub fn fetch_registry_sync() -> RegistryToolsResponse {
        match RegistryCache::instance().get_registry_tools_sync() {
            Ok(registry) => registry,
            Err(e) => {
                // Log the error and return an empty registry
                eprintln!("Failed to fetch registry: {}", e);
                RegistryToolsResponse {
                    count: 0,
                    version: 0,
                    categories: HashMap::new(),
                    tags: HashMap::new(),
                    tools: vec![],
                }
            }
        }
    }

    /// Get a specific tool from the registry by ID
    pub async fn get_tool_by_id(tool_id: &str) -> Result<RegistryTool, ErrorResponse> {
        let registry = Self::fetch_registry().await?;

        let tool = registry
            .tools
            .iter()
            .find(|tool| tool.id == tool_id)
            .cloned();

        match tool {
            Some(tool) => Ok(tool),
            None => Err(ErrorResponse {
                code: -32000,
                message: format!("Tool '{}' not found in registry", tool_id),
            }),
        }
    }

    /// Get a registry tools response with installation status marked
    pub async fn get_registry_with_install_status(
        installed_tools: &HashMap<String, String>,
    ) -> Result<Value, Value> {
        match Self::fetch_registry().await {
            Ok(registry) => {
                let mut registry_value = serde_json::to_value(registry).unwrap_or(json!({"tools": []}));

                // Mark installation status for each tool
                if let Some(tools) = registry_value.get_mut("tools").and_then(|t| t.as_array_mut()) {
                    for tool in tools {
                        if let Some(name) = tool.get("name").and_then(|n| n.as_str()) {
                            let is_installed = installed_tools.contains_key(name);
                            if let Some(obj) = tool.as_object_mut() {
                                obj.insert("installed".to_string(), json!(is_installed));
                            }
                        }
                    }
                }

                Ok(registry_value)
            }
            Err(error) => Err(serde_json::to_value(error).unwrap()),
        }
    }

    /// Update the registry cache
    pub async fn update_registry_cache() -> Result<(), ErrorResponse> {
        RegistryCache::instance().update_registry_cache().await?;
        Ok(())
    }

    /// Force update the registry cache synchronously
    pub fn update_registry_cache_sync() -> Result<(), String> {
        let result = RegistryCache::instance().get_registry_tools_sync();
        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    pub fn get_tool_by_id_sync(tool_id: &str) -> Result<RegistryTool, String> {
        let registry = Self::fetch_registry_sync();
        let tool = registry
            .tools
            .iter()
            .find(|tool| tool.id == tool_id)
            .cloned();

        match tool {
            Some(tool) => Ok(tool),
            None => Err(format!("Tool '{}' not found in registry", tool_id)),
        }
    }
}
