use axum::response::IntoResponse;
use axum::{
    http::StatusCode,
    routing::{get, post},
    Extension, Json, Router,
};
use log::{error, info};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::Arc;

use crate::domain::services::ToolService;

#[derive(Deserialize)]
pub struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Value,
    method: String,
    params: Option<Value>,
}

#[derive(Serialize)]
pub struct JsonRpcResponse {
    jsonrpc: String,
    id: Value,
    result: Option<Value>,
    error: Option<JsonRpcError>,
}

#[derive(Serialize)]
pub struct JsonRpcError {
    code: i32,
    message: String,
    data: Option<Value>,
}

#[derive(Clone)]
pub struct HttpState {
    pub tool_service: Arc<ToolService>,
}

pub async fn start_http_server(tool_service: Arc<ToolService>) {
    // Create the HTTP state
    let http_state = Arc::new(HttpState { tool_service });

    let app = Router::new()
        .route("/mcp-proxy", post(handle_mcp_request))
        .route("/health", get(health_check))
        .layer(Extension(http_state));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    info!("MCP HTTP server starting on {}", addr);

    tokio::spawn(async move {
        match axum::serve(tokio::net::TcpListener::bind(&addr).await.unwrap(), app).await {
            Ok(_) => info!("MCP HTTP server terminated normally"),
            Err(e) => error!("MCP HTTP server error: {}", e),
        }
    });
}

async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "MCP Server is running!")
}

async fn handle_mcp_request(
    Extension(state): Extension<Arc<HttpState>>,
    Json(request): Json<JsonRpcRequest>,
) -> Json<JsonRpcResponse> {
    info!("Received MCP request: method={}", request.method);

    let tool_service = &state.tool_service;

    let result = match request.method.as_str() {
        "tools/list" => match tool_service.list_tools().await {
            Ok(tools) => Ok(json!({ "tools": tools })),
            Err(e) => Err(json!({"code": -32603, "message": e})),
        },
        "tools/call" => {
            if let Some(params) = request.params {
                // Extract tool name and arguments from params
                let tool_name = match params.get("name").and_then(|v| v.as_str()) {
                    Some(name) => name,
                    None => {
                        return Json(JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id: request.id,
                            result: None,
                            error: Some(JsonRpcError {
                                code: -32602,
                                message: "Missing name in parameters".to_string(),
                                data: None,
                            }),
                        });
                    }
                };

                let arguments = match params.get("arguments") {
                    Some(args) => args.clone(),
                    None => json!({}),
                };

                // Call the domain service to execute the tool
                match tool_service
                    .execute_proxy_tool(crate::models::models::ToolExecutionRequest {
                        tool_id: tool_name.to_string(),
                        parameters: arguments,
                    })
                    .await
                {
                    Ok(response) => {
                        if response.success {
                            Ok(json!({
                                "content": [
                                    {
                                        "type": "text",
                                        "text": response.result.unwrap_or(json!("No result")).to_string()
                                    }
                                ]
                            }))
                        } else {
                            Err(json!({
                                "code": -32000,
                                "message": response.error.unwrap_or_else(|| "Unknown error".to_string())
                            }))
                        }
                    }
                    Err(e) => Err(json!({
                        "code": -32000,
                        "message": format!("Tool execution error: {}", e)
                    })),
                }
            } else {
                Err(json!({
                    "code": -32602,
                    "message": "Invalid params - missing parameters for tool invocation"
                }))
            }
        }
        "prompts/list" => Ok(json!({ "prompts": [] })),
        "resources/list" => Ok(json!({ "resources": [] })),
        "resources/read" => Err(json!({
            "code": -32601,
            "message": "Resource reading not implemented yet"
        })),
        "prompts/get" => Err(json!({
            "code": -32601,
            "message": "Prompt retrieval not implemented yet"
        })),
        _ => Err(json!({
            "code": -32601,
            "message": format!("Method not found: {}", request.method)
        })),
    };

    match result {
        Ok(result_value) => Json(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(result_value),
            error: None,
        }),
        Err(error_value) => {
            let code = error_value
                .get("code")
                .and_then(|v| v.as_i64())
                .unwrap_or(-32000) as i32;

            let message = error_value
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error")
                .to_string();

            Json(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: None,
                error: Some(JsonRpcError {
                    code,
                    message,
                    data: None,
                }),
            })
        }
    }
}
