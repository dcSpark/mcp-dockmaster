use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use log::{info, error};
use axum::{
    routing::{get, post},
    http::StatusCode,
    Json, Router, Extension,
};
use axum::response::IntoResponse;
use crate::features::mcp_proxy::MCPState;

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

pub async fn start_http_server(state: Arc<RwLock<MCPState>>) {
    let app = Router::new()
        .route("/mcp-proxy", post(handle_mcp_request))
        .route("/health", get(health_check))
        .layer(Extension(state));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    info!("MCP HTTP server starting on {}", addr);
    
    tokio::spawn(async move {
        match axum::serve(
            tokio::net::TcpListener::bind(&addr).await.unwrap(),
            app
        ).await
        {
            Ok(_) => info!("MCP HTTP server terminated normally"),
            Err(e) => error!("MCP HTTP server error: {}", e),
        }
    });
}

async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "MCP Server is running!")
}

async fn handle_mcp_request(
    Extension(state): Extension<Arc<RwLock<MCPState>>>,
    Json(request): Json<JsonRpcRequest>,
) -> Json<JsonRpcResponse> {
    info!("Received MCP request: method={}", request.method);
    
    let result = match request.method.as_str() {
        "tools/list" => handle_list_tools(state).await,
        "tools/call" => {
            if let Some(params) = request.params {
                handle_invoke_tool(state, params).await
            } else {
                Err(json!({
                    "code": -32602,
                    "message": "Invalid params - missing parameters for tool invocation"
                }))
            }
        },
        "prompts/list" => handle_list_prompts().await,
        "resources/list" => handle_list_resources().await,
        "resources/read" => {
            if let Some(params) = request.params {
                handle_read_resource(params).await
            } else {
                Err(json!({
                    "code": -32602,
                    "message": "Invalid params - missing parameters for resource reading"
                }))
            }
        },
        "prompts/get" => {
            if let Some(params) = request.params {
                handle_get_prompt(params).await
            } else {
                Err(json!({
                    "code": -32602,
                    "message": "Invalid params - missing parameters for prompt retrieval"
                }))
            }
        },
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
            let code = error_value.get("code")
                .and_then(|v| v.as_i64())
                .unwrap_or(-32000) as i32;
                
            let message = error_value.get("message")
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

async fn handle_list_tools(state: Arc<RwLock<MCPState>>) -> Result<Value, Value> {
    let mcp_state = state.read().await;
    let registry = mcp_state.tool_registry.read().await;
    
    let mut all_tools = Vec::new();
    
    for (server_id, tools) in &registry.server_tools {
        for tool in tools {
            let mut tool_info = serde_json::Map::new();
            
            if let Some(obj) = tool.as_object() {
                for (key, value) in obj {
                    tool_info.insert(key.clone(), value.clone());
                }
            }
            
            tool_info.insert("server_id".to_string(), json!(server_id));
            
            // let tool_id = tool.get("id").and_then(|v| v.as_str()).unwrap_or("main");
            // let tool_name = tool.get("name").and_then(|v| v.as_str()).unwrap_or("tool_name");
            
            // let proxy_id = format!("{}:{}", server_id, tool_id);

            // tool_info.insert("name".to_string(), json!(tool_name));
            
            if !tool_info.contains_key("description") {
                tool_info.insert("description".to_string(), json!("Tool from server"));
            }
            
            if !tool_info.contains_key("inputSchema") {
                tool_info.insert("inputSchema".to_string(), json!({
                    "type": "object",
                    "properties": {}
                }));
            }
            
            all_tools.push(json!(tool_info));
        }
    }
    
    Ok(json!({
        "tools": all_tools
    }))
}

async fn handle_list_prompts() -> Result<Value, Value> {
    Ok(json!({
        "prompts": []
    }))
}

async fn handle_list_resources() -> Result<Value, Value> {
    Ok(json!({
        "resources": []
    }))
}

async fn handle_read_resource(_params: Value) -> Result<Value, Value> {
    Err(json!({
        "code": -32601,
        "message": "Resource reading not implemented yet"
    }))
}

async fn handle_get_prompt(_params: Value) -> Result<Value, Value> {
    Err(json!({
        "code": -32601,
        "message": "Prompt retrieval not implemented yet"
    }))
}

async fn handle_invoke_tool(state: Arc<RwLock<MCPState>>, params: Value) -> Result<Value, Value> {
    let tool_name = match params.get("name").and_then(|v| v.as_str()) {
        Some(name) => name,
        None => return Err(json!({
            "code": -32602,
            "message": "Missing name in parameters"
        })),
    };
    
    let arguments = match params.get("arguments") {
        Some(args) => args.clone(),
        None => json!({}),
    };
    
    let mcp_state = state.read().await;
    let mut registry = mcp_state.tool_registry.write().await;
    
    // Find which server has the requested tool
    let mut server_id = None;
    
    for (sid, tools) in &registry.server_tools {
        for tool in tools {
            if let Some(tool_id) = tool.get("id").and_then(|v| v.as_str()) {
                if tool_id == tool_name {
                    server_id = Some(sid.clone());
                    break;
                }
            }
            
            // Also check by name if id doesn't match
            if let Some(name) = tool.get("name").and_then(|v| v.as_str()) {
                if name == tool_name {
                    server_id = Some(sid.clone());
                    break;
                }
            }
        }
        
        if server_id.is_some() {
            break;
        }
    }
    
    let server_id = match server_id {
        Some(id) => id,
        None => return Err(json!({
            "code": -32602,
            "message": format!("No server found with tool: {}", tool_name)
        })),
    };

    match registry.execute_tool(&server_id, tool_name, arguments).await {
        Ok(result) => {
            Ok(json!({
                "content": [
                    {
                        "type": "text",
                        "text": result.to_string()
                    }
                ]
            }))
        },
        Err(e) => Err(json!({
            "code": -32000,
            "message": format!("Tool execution error: {}", e)
        })),
    }
}