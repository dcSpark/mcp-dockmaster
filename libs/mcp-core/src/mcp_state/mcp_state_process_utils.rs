use std::collections::HashMap;

use serde_json::Value;
use tokio::process::Child;

use crate::{
    mcp_protocol::initialize_server_connection,
    types::{ToolConfiguration, ToolEnvironment, ToolId, ToolType},
    SpawnedProcess,
};

/// Spawn an MCP server process using DMProcess
pub async fn spawn_process(
    configuration: &Value,
    tool_id: &str,
    tool_type: &str,
    env_vars: Option<&HashMap<String, String>>,
) -> Result<
    (
        Child,
        tokio::process::ChildStdin,
        tokio::process::ChildStdout,
    ),
    String,
> {
    let command = configuration
        .get("command")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Configuration missing 'command' field or not a string".to_string())?;

    let args = configuration
        .get("args")
        .and_then(|v| v.as_array())
        .map(|args| {
            args.iter()
                .filter_map(|arg| arg.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let config = ToolConfiguration {
        command: Some(command.to_string()),
        args: Some(args),
        env: env_vars.map(|vars| {
            vars.iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        ToolEnvironment {
                            description: "".to_string(),
                            default: Some(v.clone()),
                            required: false,
                        },
                    )
                })
                .collect()
        }),
    };

    let tool_type = match tool_type {
        "node" => ToolType::Node,
        "python" => ToolType::Python,
        "docker" => ToolType::Docker,
        _ => return Err(format!("Unsupported tool type: {}", tool_type)),
    };

    let tool_id = ToolId::new(tool_id.to_string());
    let mut dm_process = SpawnedProcess::new(&tool_id, &tool_type, &config, env_vars).await?;
    let _ = initialize_server_connection(
        tool_id.as_str(),
        &mut dm_process.stdin,
        &mut dm_process.stdout,
    )
    .await;
    Ok((dm_process.child, dm_process.stdin, dm_process.stdout))
}

/// Kill a running process
pub async fn kill_process(process: &mut Child) -> Result<(), String> {
    match process.kill().await {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Failed to kill process: {}", e)),
    }
}
