use axum::response::IntoResponse;
use axum::{http::StatusCode, Extension, Json};
use log::info;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::core::mcp_core::MCPCore;
use crate::core::mcp_core_proxy_ext::McpCoreProxyExt;
use crate::types::ToolRegistrationRequest;

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

pub async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "MCP Server is running!")
}

pub async fn handle_mcp_request(
    Extension(mcp_core): Extension<MCPCore>,
    Json(request): Json<JsonRpcRequest>,
) -> Json<JsonRpcResponse> {
    info!("Received MCP request: method={}", request.method);

    let result = match request.method.as_str() {
        "tools/list" => handle_list_tools(mcp_core).await,
        "tools/call" => {
            if let Some(params) = request.params {
                handle_invoke_tool(mcp_core, params).await
            } else {
                Err(json!({
                    "code": -32602,
                    "message": "Invalid params - missing parameters for tool invocation"
                }))
            }
        }
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
        }
        "prompts/get" => {
            if let Some(params) = request.params {
                handle_get_prompt(params).await
            } else {
                Err(json!({
                    "code": -32602,
                    "message": "Invalid params - missing parameters for prompt retrieval"
                }))
            }
        }
        "registry/install" => {
            if let Some(params) = request.params {
                handle_register_tool(mcp_core, params).await
            } else {
                Err(json!({
                    "code": -32602,
                    "message": "Invalid params - missing parameters for tool registration"
                }))
            }
        }
        "registry/list" => handle_list_all_tools(mcp_core).await,
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

async fn handle_list_tools(mcp_core: MCPCore) -> Result<Value, Value> {
    let mcp_state = mcp_core.mcp_state.read().await;
    let server_tools = mcp_state.server_tools.read().await;

    let mut all_tools = Vec::new();

    for (server_id, tools) in &*server_tools {
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
                tool_info.insert(
                    "inputSchema".to_string(),
                    json!({
                        "type": "object",
                        "properties": {}
                    }),
                );
            }

            all_tools.push(json!(tool_info));
        }
    }

    Ok(json!({
        "tools": all_tools
    }))
}

async fn handle_register_tool(mcp_core: MCPCore, params: Value) -> Result<Value, Value> {
    println!("handle_register_tool: trying to register tool {:?}", params);
    let registry = fetch_tool_from_registry().await?;
    let tools = registry.get("tools").unwrap().as_array().unwrap();

    let tool = tools
        .iter()
        .find(|tool| tool.get("name").unwrap().as_str() == params.get("tool").unwrap().as_str());

    match tool {
        Some(tool) => {
            println!("[PRE] handle_register_tool: tool {:?}", tool);

            let tool_name = tool
                .get("name")
                .unwrap()
                .as_str()
                .unwrap_or("error")
                .to_string();

            let description = tool
                .get("description")
                .unwrap()
                .as_str()
                .unwrap_or_default()
                .to_string();

            let tool_type = tool
                .get("runtime")
                .unwrap_or(&json!("error"))
                .as_str()
                .unwrap_or("error")
                .to_string();

            let configuration = tool.get("config").map(|config| {
                json!({
                    "command": config.get("command").unwrap_or(&json!("error")),
                    "args": config.get("args").unwrap_or(&json!([]))
                })
            });

            let distribution = tool.get("distribution").map(|dist| {
                json!({
                    "type": dist.get("type").unwrap_or(&json!("error")),
                    "package": dist.get("package").unwrap_or(&json!("error"))
                })
            });

            let tool = ToolRegistrationRequest {
                tool_name,
                description,
                tool_type,
                configuration,
                distribution,
            };

            println!("[POST] handle_register_tool: tool {:?}", tool);
            let r = mcp_core.register_tool(tool).await;
            // Asume installation is successful
            println!("[INSTALLATION] handle_register_tool: r {:?}", r);
            Ok(json!({
                "code": 0,
                "message": "Tool installed successfully"
            }))
        }
        None => Err(json!({
            "code": -32602,
            "message": format!("Tool not found: {:?}", params.get("name"))
        })),
    }
}

async fn fetch_tool_from_registry() -> Result<Value, Value> {
    // Fetch tools from remote URL
    let tools_url = "https://pub-790f7c5dc69a482998b623212fa27446.r2.dev/db.v0.json";

    let client = reqwest::Client::builder().build().unwrap_or_default();

    let response = client
        .get(tools_url)
        .header("Accept-Encoding", "gzip")
        .header("User-Agent", "MCP-Core/1.0")
        .send()
        .await
        .map_err(|e| {
            json!({
                "code": -32000,
                "message": format!("Failed to fetch tools from registry: {}", e)
            })
        })?;

    let tools: Vec<Value> = response.json().await.map_err(|e| {
        json!({
            "code": -32000,
            "message": format!("Failed to parse tools from registry: {}", e)
        })
    })?;

    let mut all_tools = Vec::new();

    for tool in tools {
        let mut tool_info = serde_json::Map::new();

        if let Some(obj) = tool.as_object() {
            for (key, value) in obj {
                tool_info.insert(key.clone(), value.clone());
            }
        }

        if !tool_info.contains_key("description") {
            tool_info.insert("description".to_string(), json!("Tool from registry"));
        }

        if !tool_info.contains_key("inputSchema") {
            tool_info.insert(
                "inputSchema".to_string(),
                json!({
                    "type": "object",
                    "properties": {}
                }),
            );
        }

        all_tools.push(json!(tool_info));
    }
    let v = json!({
        "tools": all_tools
    });
    Ok(v)
}

async fn handle_list_all_tools(mcp_core: MCPCore) -> Result<Value, Value> {
    let mcp_state = mcp_core.mcp_state.read().await;
    let registry = mcp_state.tool_registry.read().await;
    let installed_tools = registry.get_all_tools()?;
    let mut registry_tools = fetch_tool_from_registry().await?;

    for tool in registry_tools
        .get_mut("tools")
        .unwrap()
        .as_array_mut()
        .unwrap()
    {
        let tool_name = tool.get("name").unwrap().as_str().unwrap();
        if installed_tools.contains_key(tool_name) {
            println!("Tool {} is installed", tool_name);
            tool.as_object_mut()
                .unwrap()
                .insert("installed".to_string(), json!(true));
        } else {
            println!("Tool {} is not installed", tool_name);
            tool.as_object_mut()
                .unwrap()
                .insert("installed".to_string(), json!(false));
        }
    }

    Ok(registry_tools)
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

async fn handle_invoke_tool(mcp_core: MCPCore, params: Value) -> Result<Value, Value> {
    let tool_name = match params.get("name").and_then(|v| v.as_str()) {
        Some(name) => name,
        None => {
            return Err(json!({
                "code": -32602,
                "message": "Missing name in parameters"
            }))
        }
    };

    let arguments = match params.get("arguments") {
        Some(args) => args.clone(),
        None => json!({}),
    };

    let mcp_state = mcp_core.mcp_state.read().await;
    let server_tools = mcp_state.server_tools.read().await;

    // Find which server has the requested tool
    let mut server_id = None;

    for (sid, tools) in &*server_tools {
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
        None => {
            return Err(json!({
                "code": -32602,
                "message": format!("No server found with tool: {}", tool_name)
            }))
        }
    };

    // Drop the server_tools lock before executing the tool
    drop(server_tools);

    match mcp_state
        .execute_tool(&server_id, tool_name, arguments)
        .await
    {
        Ok(result) => Ok(json!({
            "content": [
                {
                    "type": "text",
                    "text": result.to_string()
                }
            ]
        })),
        Err(e) => Err(json!({
            "code": -32000,
            "message": format!("Tool execution error: {}", e)
        })),
    }
}
