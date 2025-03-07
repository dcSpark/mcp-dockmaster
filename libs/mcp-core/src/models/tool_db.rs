use crate::schema::{tools, tool_env};
use diesel::prelude::*;

/// This struct corresponds to a row in the `tools` table.
#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = tools)]
pub struct DBTool {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tool_type: String,
    pub enabled: bool,
    pub entry_point: Option<String>,
    pub command: Option<String>,
    pub args: Option<String>,
    pub distribution_type: Option<String>,
    pub distribution_package: Option<String>,
}

/// For inserting a new row into the `tools` table
#[derive(Debug, Insertable)]
#[diesel(table_name = tools)]
pub struct NewTool<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub description: &'a str,
    pub tool_type: &'a str,
    pub enabled: bool,
    pub entry_point: Option<&'a str>,
    pub command: Option<&'a str>,
    pub args: Option<&'a str>,
    pub distribution_type: Option<&'a str>,
    pub distribution_package: Option<&'a str>,
}

/// For updating an existing row in the `tools` table
#[derive(Debug, AsChangeset)]
#[diesel(table_name = tools)]
pub struct UpdateTool<'a> {
    pub name: Option<&'a str>,
    pub description: Option<&'a str>,
    pub tool_type: Option<&'a str>,
    pub enabled: Option<bool>,
    pub entry_point: Option<Option<&'a str>>,
    pub command: Option<Option<&'a str>>,
    pub args: Option<Option<&'a str>>,
    pub distribution_type: Option<Option<&'a str>>,
    pub distribution_package: Option<Option<&'a str>>,
}

/// This struct corresponds to a row in the `tool_env` table.
#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = tool_env)]
pub struct DBToolEnv {
    pub tool_id: String,
    pub env_key: String,
    pub env_value: String,
    pub env_description: String,
    pub env_required: bool,
}

/// For inserting a new row into the `tool_env` table
#[derive(Debug, Insertable)]
#[diesel(table_name = tool_env)]
pub struct NewToolEnv {
    pub tool_id: String,
    pub env_key: String,
    pub env_value: String,
    pub env_description: String,
    pub env_required: bool,
}

/// For updating an existing row in the `tool_env` table
#[derive(Debug, AsChangeset)]
#[diesel(table_name = tool_env)]
#[diesel(primary_key(tool_id, env_key))]
pub struct UpdateToolEnv<'a> {
    pub env_value: Option<&'a str>,
    pub env_description: Option<&'a str>,
    pub env_required: Option<bool>,
}
