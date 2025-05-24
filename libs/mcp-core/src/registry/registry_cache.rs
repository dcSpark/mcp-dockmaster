use lazy_static::lazy_static;
use log::{info, warn};
use reqwest;
use serde_json::Value;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use crate::models::types::RegistryToolsResponse;

// Cache duration constant (10 minute)
const CACHE_DURATION: Duration = Duration::from_secs(600);

// Cache structure to store registry data and timestamp
struct RegistryCacheEntry {
    data: Option<Value>,
    timestamp: Option<Instant>,
}

impl RegistryCacheEntry {
    fn new() -> Self {
        Self {
            data: None,
            timestamp: None,
        }
    }

    fn is_valid(&self) -> bool {
        if let (Some(data), Some(timestamp)) = (&self.data, self.timestamp) {
            // Check that the timestamp is recent and data is not empty/null
            if timestamp.elapsed() < CACHE_DURATION && !data.is_null() {
                // Verify that the data appears to have the expected structure
                if let Some(tools) = data.get("tools") {
                    if tools.is_array() {
                        return true;
                    }
                }
            }
        }
        false
    }
}

// Cache structure that provides both sync and async access
pub struct RegistryCache {
    // Use RwLock for sync access
    sync_cache: Arc<RwLock<RegistryCacheEntry>>,
    // Use Mutex for async access
    async_cache: Mutex<()>,
}

lazy_static! {
    static ref REGISTRY_CACHE_INSTANCE: RegistryCache = RegistryCache::new();
}

impl RegistryCache {
    fn new() -> Self {
        Self {
            sync_cache: Arc::new(RwLock::new(RegistryCacheEntry::new())),
            async_cache: Mutex::new(()),
        }
    }

    // Get instance of the cache singleton
    pub fn instance() -> &'static Self {
        &REGISTRY_CACHE_INSTANCE
    }

    // Get registry tools from cache or fetch if needed (sync version)
    pub fn get_registry_tools_sync(&self) -> Result<RegistryToolsResponse, String> {
        // Check if we have a valid cache
        {
            let cache_read = self.sync_cache.read().unwrap();
            if cache_read.is_valid() {
                if let Some(data) = &cache_read.data {
                    match serde_json::from_value::<RegistryToolsResponse>(data.clone()) {
                        Ok(response) => return Ok(response),
                        Err(e) => warn!("Failed to parse cached registry data: {}", e),
                    }
                }
            }
        }

        // Cache is invalid or corrupted, need to fetch fresh data
        // This is a blocking call in a synchronous context - not ideal but necessary
        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => return Err(format!("Failed to create Tokio runtime: {}", e)),
        };

        let result = rt.block_on(async {
            // Update the cache and return data
            self.update_registry_cache().await
        });

        match result {
            Ok(response) => Ok(response),
            Err(e) => Err(e),
        }
    }

    // Get registry tools from cache or fetch if needed (async version)
    pub async fn get_registry_tools(&self) -> Result<RegistryToolsResponse, String> {
        // Check if we have a valid cache first (using read lock)
        {
            let cache_read = self.sync_cache.read().unwrap();
            if cache_read.is_valid() {
                if let Some(data) = &cache_read.data {
                    match serde_json::from_value::<RegistryToolsResponse>(data.clone()) {
                        Ok(response) => return Ok(response),
                        Err(_) => {
                            // Cache data is corrupted, we'll refresh it
                        }
                    }
                }
            }
        }

        // Cache is invalid or doesn't exist, fetch fresh data
        self.update_registry_cache().await
    }

    // Update the registry cache with fresh data
    pub async fn update_registry_cache(&self) -> Result<RegistryToolsResponse, String> {
        // Lock to prevent multiple simultaneous fetches
        let _guard = self.async_cache.lock().await;

        // Double-check if cache was updated while we were waiting for the lock
        {
            let cache_read = self.sync_cache.read().unwrap();
            if cache_read.is_valid() {
                if let Some(data) = &cache_read.data {
                    match serde_json::from_value::<RegistryToolsResponse>(data.clone()) {
                        Ok(response) => return Ok(response),
                        Err(_) => {
                            // Cache data is corrupted, will refresh it
                        }
                    }
                }
            }
        }

        // Fetch tools from remote URL
        // All Tools: Stable & Unstable
        let tools_url =
            "https://pub-5e2d77d67aac45ef811998185d312005.r2.dev/registry/registry.all.json";

        info!("Fetching registry data from {}", tools_url);
        let client = reqwest::Client::builder().build().unwrap_or_default();

        let response = client
            .get(tools_url)
            .header("Accept-Encoding", "gzip")
            .header("User-Agent", "MCP-Core/1.0")
            .send()
            .await
            .map_err(|e| format!("Failed to fetch tools from registry: {}", e))?;

        let raw = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse tools from registry: {}", e))?;

        let tool_wrapper: RegistryToolsResponse = serde_json::from_value(raw)
            .map_err(|e| format!("Failed to parse tools from registry: {}", e))?;

        info!("Fetched {} tools from registry", tool_wrapper.tools.len());

        // Update the cache with new data
        {
            let mut cache_write = self.sync_cache.write().unwrap();
            cache_write.data = Some(serde_json::to_value(&tool_wrapper).unwrap_or_default());
            cache_write.timestamp = Some(Instant::now());
        }

        Ok(tool_wrapper)
    }
}
