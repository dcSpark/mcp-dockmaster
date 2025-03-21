use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct Tools {
    pub tools: Vec<Tool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Tool {
    pub description: String,
    pub short_description: String,
    pub input_schema: InputSchema,
    pub name: String,
    pub server_id: String,
    pub installed: bool,
    pub categories: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InputSchema {
    pub description: String,
    pub properties: Properties,
    pub required: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(rename = "type")]
    pub schema_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_properties: Option<bool>,
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Properties {
    #[serde(flatten)]
    pub properties: HashMap<String, PropertyDefinition>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PropertyDefinition {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclusive_maximum: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclusive_minimum: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<TypeDefinition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_properties: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub all_of: Option<Vec<PropertyDefinition>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TypeDefinition {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegistryTool {
    pub id: String,
    pub name: String,
    pub description: String,
    pub short_description: String,
    pub publisher: Publisher,
    pub is_official: bool,
    pub source_url: String,
    pub distribution: Distribution,
    pub license: String,
    pub runtime: String,
    pub config: Config,
    pub categories: Vec<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub command: String,
    pub args: Vec<String>,
    pub env: serde_json::Value, // Using serde_json::Value for 'any' type
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Distribution {
    #[serde(rename = "type")]
    pub distribution_type: String,
    pub package: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Publisher {
    pub id: String,
    pub name: String,
    pub url: String,
}

/// Definition for a tool the client can call.
#[derive(Debug, Serialize, Deserialize)]
pub struct OfficialTool {
    /// The name of the tool.
    pub name: String,
    /// A human-readable description of the tool.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// A JSON Schema object defining the expected parameters for the tool.
    pub input_schema: OfficialToolInputSchema,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OfficialToolInputSchema {
    #[serde(rename = "type")]
    pub schema_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
}
