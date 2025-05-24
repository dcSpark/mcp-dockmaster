use std::time::Duration;

use lazy_static::lazy_static;
use serde_json::Value;
use tokio::{sync::Mutex, time::Instant};

use crate::types::RegistryToolsResponse;

struct RegistryCache {
    data: Option<Value>,
    timestamp: Option<Instant>,
}

// Initialize the static cache with lazy_static
lazy_static! {
    static ref REGISTRY_CACHE: Mutex<RegistryCache> = Mutex::new(RegistryCache {
        data: None,
        timestamp: None,
    });
}

// Cache duration constant (1 minutes)
const CACHE_DURATION: Duration = Duration::from_secs(60);

pub async fn fetch_tool_from_registry() -> Result<RegistryToolsResponse, rmcp::Error> {
    // Check if we have a valid cache
    let use_cache = {
        let cache = REGISTRY_CACHE.lock().await;
        if let (Some(data), Some(timestamp)) = (&cache.data, cache.timestamp) {
            if timestamp.elapsed() < CACHE_DURATION {
                // Cache is still valid
                serde_json::from_value::<RegistryToolsResponse>(data.clone()).ok()
            } else {
                None
            }
        } else {
            None
        }
    };

    // If we have valid cached data, return it
    if let Some(cached_data) = use_cache {
        return Ok(cached_data);
    }

    // Cache is invalid or doesn't exist, fetch fresh data
    // Fetch tools from remote URL
    // All Tools: Stable & Unstable
    let tools_url =
        "https://pub-5e2d77d67aac45ef811998185d312005.r2.dev/registry/registry.all.json";
    // Stable Only
    // let tools_url =
    //     "https://pub-5e2d77d67aac45ef811998185d312005.r2.dev/registry/registry.stable.json";
    // Unstable Only
    // let tools_url =
    //     "https://pub-5e2d77d67aac45ef811998185d312005.r2.dev/registry/registry.unstable.json";

    let client = reqwest::Client::builder().build().unwrap_or_default();

    let response = client
        .get(tools_url)
        .header("Accept-Encoding", "gzip")
        .header("User-Agent", "MCP-Core/1.0")
        .send()
        .await
        .map_err(|e| {
            rmcp::Error::internal_error(format!("Failed to fetch tools from registry: {}", e), None)
        })?;

    let raw = response.json().await.map_err(|e| {
        rmcp::Error::internal_error(format!("Failed to parse tools from registry: {}", e), None)
    })?;

    let tool_wrapper: RegistryToolsResponse = serde_json::from_value(raw).map_err(|e| {
        rmcp::Error::internal_error(format!("Failed to parse tools from registry: {}", e), None)
    })?;

    println!("[TOOLS] found # tools {:?}", tool_wrapper.tools.len());

    // let result = RegistryToolsResponse { tools };

    // Update the cache with new data
    {
        let mut cache = REGISTRY_CACHE.lock().await;
        cache.data = Some(serde_json::to_value(&tool_wrapper).unwrap_or_default());
        cache.timestamp = Some(Instant::now());
    }

    Ok(tool_wrapper)
}
