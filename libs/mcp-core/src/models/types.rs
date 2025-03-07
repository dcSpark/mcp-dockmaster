use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolId(String);

impl ToolId {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Hash for ToolId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl PartialEq for ToolId {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for ToolId {}

impl fmt::Display for ToolId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolType {
    #[default]
    Node,
    Python,
    Docker,
}

#[derive(Debug, Serialize)]
pub struct ToolConfigUpdateResponse {
    pub success: bool,
    pub message: String,
}
/// Tool configuration for command and arguments
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolConfiguration {
    pub command: String,
    #[serde(default)]
    pub args: Option<Vec<String>>,
    #[serde(default)]
    pub env: Option<HashMap<String, ToolEnvironment>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolEnvironment {
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub default: Option<String>,
    #[serde(default)]
    pub required: bool,
}

/// Tool config for environment variables and optional command
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolConfig {
    #[serde(default)]
    pub env: Option<HashMap<String, String>>,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub args: Option<Vec<String>>,
}

/// Tool definition with all properties
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub tool_type: String,
    #[serde(default)]
    pub entry_point: Option<String>,
    #[serde(default)]
    pub configuration: Option<ToolConfiguration>,
    #[serde(default)]
    pub distribution: Option<Distribution>,
    #[serde(default)]
    pub config: Option<ToolConfig>,
    #[serde(default)]
    #[serde(rename = "authentication")]
    pub env_configs: Option<EnvConfigs>,
}

// TODO: Add these to the ToolRegistry struct
//     pub tools: HashMap<ToolId, ToolMetadata>,
//     pub processes: HashMap<ToolId, Option<ProcessManager>>,
//     pub server_tools: HashMap<ToolId, Vec<Value>>,

/// MCP tool registration request
#[derive(Debug, Deserialize)]
pub struct ToolRegistrationRequest {
    pub tool_name: String,
    pub description: String,
    #[serde(rename = "authentication")]
    pub env_configs: Option<Value>,
    pub tool_type: String, // "node", "python", "docker"
    pub configuration: Option<Value>,
    pub distribution: Option<Value>,
}

/// MCP tool registration response
#[derive(Debug, Serialize)]
pub struct ToolRegistrationResponse {
    pub success: bool,
    pub message: String,
    pub tool_id: Option<String>,
}

/// MCP tool execution request
#[derive(Deserialize)]
pub struct ToolExecutionRequest {
    pub tool_id: String,
    pub parameters: Value,
}

/// MCP tool execution response
#[derive(Serialize)]
pub struct ToolExecutionResponse {
    pub success: bool,
    pub result: Option<Value>,
    pub error: Option<String>,
}

/// MCP tool update request
#[derive(Deserialize)]
pub struct ToolUpdateRequest {
    pub tool_id: String,
    pub enabled: bool,
}

/// MCP tool update response
#[derive(Serialize)]
pub struct ToolUpdateResponse {
    pub success: bool,
    pub message: String,
}

/// MCP tool config update request
#[derive(Deserialize)]
pub struct ToolConfigUpdateRequest {
    pub tool_id: String,
    pub config: ToolConfig,
}

/// MCP tool uninstall request
#[derive(Deserialize)]
pub struct ToolUninstallRequest {
    pub tool_id: String,
}

/// MCP tool uninstall response
#[derive(Serialize)]
pub struct ToolUninstallResponse {
    pub success: bool,
    pub message: String,
}

/// MCP server discovery request
#[derive(Deserialize)]
pub struct DiscoverServerToolsRequest {
    pub server_id: String,
}

/// MCP server discovery response
#[derive(Serialize)]
pub struct DiscoverServerToolsResponse {
    pub success: bool,
    pub tools: Option<Vec<Value>>,
    pub error: Option<String>,
}

/// Distribution information for a tool
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Distribution {
    pub r#type: String,
    pub package: String,
}

/// Environment configuration for a tool
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnvConfigs {
    #[serde(default)]
    pub env: Option<HashMap<String, String>>,
}
