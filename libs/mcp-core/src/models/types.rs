use rmcp::model::{JsonObject, Tool};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerId(String);

impl ServerId {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Hash for ServerId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl PartialEq for ServerId {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for ServerId {}

impl fmt::Display for ServerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Display for ServerStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServerStatus::Running => write!(f, "Running"),
            ServerStatus::Stopped => write!(f, "Stopped"),
            ServerStatus::Starting => write!(f, "Starting"),
            ServerStatus::Error(msg) => write!(f, "Error: {}", msg),
        }
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

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServerStatus {
    Running,
    Stopped,
    Starting,
    #[serde(
        serialize_with = "serialize_error",
        deserialize_with = "deserialize_error"
    )]
    Error(String),
}

#[derive(Debug, Serialize)]
pub struct ToolConfigUpdateResponse {
    pub success: bool,
    pub message: String,
}

/// Tool configuration for command and arguments
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerConfiguration {
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub args: Option<Vec<String>>,
    #[serde(default)]
    pub env: Option<HashMap<String, ServerEnvironment>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerEnvironment {
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub default: Option<String>,
    #[serde(default)]
    pub required: bool,
}

// ToolConfig struct has been removed and merged into ToolConfiguration

/// Server definition with all properties
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerDefinition {
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub tools_type: String,
    #[serde(default)]
    pub entry_point: Option<String>,
    #[serde(default)]
    pub configuration: Option<ServerConfiguration>,
    #[serde(default)]
    pub distribution: Option<Distribution>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuntimeServer {
    #[serde(flatten)]
    pub definition: ServerDefinition,
    pub id: ServerId,
    pub status: ServerStatus,
    pub tool_count: usize,
}

/// MCP server registration request
#[derive(Debug, Deserialize)]
pub struct ServerRegistrationRequest {
    pub server_id: String,
    pub server_name: String,
    pub description: String,
    pub tools_type: String, // "node", "python", "docker"
    pub configuration: Option<ServerConfiguration>,
    pub distribution: Option<Distribution>,
}

#[derive(Debug, Deserialize)]
pub struct IsProcessRunningRequest {
    pub process_name: String,
}

/// MCP tool registration response
#[derive(Debug, Serialize)]
pub struct ServerRegistrationResponse {
    pub success: bool,
    pub message: String,
    pub tool_id: Option<String>,
}

/// MCP tool execution request
#[derive(Deserialize)]
pub struct ToolExecutionRequest {
    pub tool_id: String,
    pub parameters: Option<Map<String, Value>>,
}

/// MCP tool execution response
#[derive(Serialize, Debug)]
pub struct ToolExecutionResponse {
    pub success: bool,
    pub result: Option<Value>,
    pub error: Option<String>,
}

/// MCP tool update request
#[derive(Deserialize)]
pub struct ServerUpdateRequest {
    pub server_id: String,
    pub enabled: bool,
}

/// MCP tool update response
#[derive(Serialize, Debug)]
pub struct ToolUpdateResponse {
    pub success: bool,
    pub message: String,
}

/// MCP tool config update request
#[derive(Deserialize)]
pub struct ServerConfigUpdateRequest {
    pub server_id: String,
    pub config: HashMap<String, String>,
}

/// MCP tool uninstall request
#[derive(Deserialize)]
pub struct ToolUninstallRequest {
    pub server_id: String,
}

/// MCP tool uninstall response
#[derive(Serialize)]
pub struct ServerUninstallResponse {
    pub success: bool,
    pub message: String,
}

/// MCP server discovery request
#[derive(Serialize, Deserialize, Debug)]
pub struct DiscoverServerToolsRequest {
    pub server_id: String,
}

/// Distribution information for a tool
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Distribution {
    pub r#type: String,
    pub package: String,
}

// Custom serializer for the Error variant to format as "Error: message"
fn serialize_error<S>(error_message: &String, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&format!("Error: {}", error_message))
}

// Custom deserializer for the Error variant that handles "Error: message" format
fn deserialize_error<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if s.starts_with("Error: ") {
        // Use a safer way to remove the prefix
        Ok(s.trim_start_matches("Error: ").to_string())
    } else {
        Ok(s) // Return as is if it doesn't have the prefix
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Publisher {
    pub id: String,
    pub name: String,
    pub url: String,
}

// #[derive(Clone, Debug, Serialize, Deserialize)]
// pub struct Config {
//     pub command: String,
//     pub args: Vec<String>,
//     pub env: HashMap<String, String>,
// }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegistryTool {
    pub id: String,
    pub name: String,
    pub description: String,
    pub short_description: String,
    pub publisher: Publisher,
    pub is_official: Option<bool>,
    pub source_url: Option<String>,
    pub distribution: Distribution,
    pub license: String,
    pub runtime: String,
    pub config: ServerConfiguration,
    pub categories: Vec<String>,
    pub tags: Vec<String>,
}

/// Response for registry tools listing
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegistryToolsResponse {
    pub count: u32,
    pub version: u64,
    pub categories: HashMap<String, u32>,
    pub tags: HashMap<String, u32>,
    pub tools: Vec<RegistryTool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConfigUpdateRequest {
    pub tool_id: String,
    pub config: HashMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct InputSchemaProperty {
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub description: String,

    // Type can be a string or array of strings
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<Value>,

    // Additional fields from the JSON example
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "exclusiveMinimum")]
    pub exclusive_minimum: Option<f64>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "exclusiveMaximum")]
    pub exclusive_maximum: Option<f64>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "additionalProperties")]
    pub additional_properties: Option<bool>,

    // Support for allOf arrays
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "allOf")]
    pub all_of: Option<Vec<Value>>,

    // Catch-all for any other properties
    #[serde(flatten)]
    #[serde(default)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub additional_fields: HashMap<String, Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InputSchema {
    #[serde(default)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub properties: HashMap<String, InputSchemaProperty>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub required: Vec<String>,

    #[serde(default)]
    pub r#type: String,

    // Additional fields from the JSON example - all made optional with default values
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "additionalProperties")]
    pub additional_properties: Option<bool>,

    #[serde(rename = "$schema")]
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,

    // Catch-all for any other properties - made optional with default value
    #[serde(flatten)]
    #[serde(default)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub additional_fields: HashMap<String, Value>,
}

impl Default for InputSchema {
    fn default() -> Self {
        Self {
            properties: HashMap::new(),
            required: Vec::new(),
            r#type: "object".to_string(),
            title: None,
            description: None,
            additional_properties: None,
            schema: None,
            additional_fields: HashMap::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerToolInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    #[serde(rename = "inputSchema")]
    pub input_schema: Option<InputSchema>,
    pub server_id: String,
    #[serde(default)]
    pub proxy_id: Option<String>,
    #[serde(default = "default_is_active")]
    pub is_active: bool,
}

fn default_is_active() -> bool {
    true
}

impl ServerToolInfo {
    /// Create a new ServerToolInfo from a JSON value
    pub fn from_value(value: Value, server_id: String) -> Result<ServerToolInfo, String> {
        // Generate id from name before deserializing
        let name = value
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or("missing name field")?;

        let id = name
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect();

        // Create mutable copy and insert the generated fields
        let mut obj = value;

        obj.as_object_mut()
            .ok_or("value must be an object")?
            .insert("id".to_string(), Value::String(id));
        obj.as_object_mut()
            .unwrap()
            .insert("server_id".to_string(), Value::String(server_id));

        // Now deserialize the complete object
        serde_json::from_value(obj).map_err(|e| e.to_string())
    }

    pub fn from_tool(tool: Tool, server_id: String) -> Result<ServerToolInfo, String> {
        let name = tool.name.clone();
        let id = name
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect();

        let input_schema: InputSchema = serde_json::from_value(
            serde_json::to_value(tool.input_schema).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;

        Ok(Self {
            id,
            server_id: server_id.clone(),
            name: tool.name.into(),
            description: tool.description.unwrap_or_default().into(),
            input_schema: Some(input_schema),
            proxy_id: None,
            is_active: true,
        })
    }

    pub fn to_tool(self) -> Result<Tool, String> {
        let input_schema: JsonObject = serde_json::to_value(self.input_schema)
            .map_err(|e| e.to_string())
            .map_err(|e| format!("failed to serialize input schema: {}", e))?
            .as_object()
            .unwrap_or(&serde_json::Map::new())
            .to_owned();
        let binding = self.name.clone();
        let name = Cow::Borrowed(&binding);
        let binding = self.description.clone();
        let description = Cow::Borrowed(&binding);

        Ok(Tool {
            name: name.into_owned().into(),
            description: Some(description.into_owned().into()),
            input_schema: Arc::new(input_schema),
            annotations: None,
        })
    }
}
