use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;
use tracing::{debug, error, info};

#[derive(Debug, Serialize)]
struct JsonRpcRequest<T> {
    jsonrpc: String,
    id: i32,
    method: String,
    params: T,
}

#[derive(Debug, Deserialize)]
struct JsonRpcResponse<T> {
    result: T,
    error: Option<JsonRpcError>,
}

#[derive(Debug, Deserialize)]
struct JsonRpcError {
    message: String,
}

/// Generic proxy function to forward requests to the target server
///
/// # Arguments
///
/// * `method` - The JSON-RPC method to call
/// * `params` - The parameters to pass to the method
///
/// # Returns
///
/// Returns the response from the target server wrapped in a Result
pub async fn request<T, R>(method: &str, params: T) -> Result<R, Box<dyn Error>>
where
    T: Serialize,
    R: for<'de> Deserialize<'de> + Serialize,
{
    debug!("proxy_request called with method: {}", method);

    let port = std::env::var("DOCKMASTER_HTTP_SERVER_PORT")
        .unwrap_or_else(|_| "11011".to_string())
        .parse::<u16>()
        .unwrap_or(11011);
    let target_server_url = format!("http://localhost:{}/mcp", port);

    info!("Target server: {}", target_server_url);
    info!("Proxying request: {} to {}", method, target_server_url);
    info!("Request params: {}", serde_json::to_string(&params)?);

    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: 1,
        method: method.to_string(),
        params,
    };

    let request_body = serde_json::to_string(&request)?;
    info!("Request body: {}", request_body);

    let client = Client::new();
    let response = client
        .post(target_server_url)
        .header("Content-Type", "application/json")
        .body(request_body)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await?;
        error!("HTTP error! status: {}, response: {}", status, text);
        return Err(format!("HTTP error! status: {}, response: {}", status, text).into());
    }

    let data: JsonRpcResponse<R> = response.json().await?;

    if let Some(error) = data.error {
        error!("Server error: {}", error.message);
        return Err(error.message.into());
    }

    info!("Received response for: {}", method);
    info!(
        "Response data: {}...",
        serde_json::to_string(&data.result)?
            .chars()
            .collect::<String>()
    );

    Ok(data.result)
}
