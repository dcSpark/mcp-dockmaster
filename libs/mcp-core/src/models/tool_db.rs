use crate::schema::{app_settings, server_env, server_tools, servers};
use diesel::prelude::*;

/// This struct corresponds to a row in the `tools` table.
#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = servers)]
pub struct DBServer {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tools_type: String,
    pub enabled: bool,
    pub entry_point: Option<String>,
    pub command: Option<String>,
    pub args: Option<String>,
    pub distribution_type: Option<String>,
    pub distribution_package: Option<String>,
}

/// For inserting a new row into the `tools` table
#[derive(Debug, Insertable)]
#[diesel(table_name = servers)]
pub struct NewServer<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub description: &'a str,
    pub tools_type: &'a str,
    pub enabled: bool,
    pub entry_point: Option<&'a str>,
    pub command: Option<&'a str>,
    pub args: Option<&'a str>,
    pub distribution_type: Option<&'a str>,
    pub distribution_package: Option<&'a str>,
}

/// For updating an existing row in the `tools` table
#[derive(Debug, AsChangeset)]
#[diesel(table_name = servers)]
pub struct UpdateServer<'a> {
    pub name: Option<&'a str>,
    pub description: Option<&'a str>,
    pub tools_type: Option<&'a str>,
    pub enabled: Option<bool>,
    pub entry_point: Option<Option<&'a str>>,
    pub command: Option<Option<&'a str>>,
    pub args: Option<Option<&'a str>>,
    pub distribution_type: Option<Option<&'a str>>,
    pub distribution_package: Option<Option<&'a str>>,
}

/// This struct corresponds to a row in the `server_env` table.
#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = server_env)]
pub struct DBServerEnv {
    pub server_id: String,
    pub env_key: String,
    pub env_value: String,
    pub env_description: String,
    pub env_required: bool,
}

/// For inserting a new row into the `server_env` table
#[derive(Debug, Insertable)]
#[diesel(table_name = server_env)]
pub struct NewServerEnv {
    pub server_id: String,
    pub env_key: String,
    pub env_value: String,
    pub env_description: String,
    pub env_required: bool,
}

/// For updating an existing row in the `server_env` table
#[derive(Debug, AsChangeset)]
#[diesel(table_name = server_env)]
#[diesel(primary_key(server_id, env_key))]
pub struct UpdateServerEnv<'a> {
    pub env_value: Option<&'a str>,
    pub env_description: Option<&'a str>,
    pub env_required: Option<bool>,
}

/// This struct corresponds to a row in the `server_tools` table.
#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = server_tools)]
pub struct DBServerTool {
    pub id: String,
    pub name: String,
    pub description: String,
    pub input_schema: Option<String>,
    pub server_id: String,
    pub proxy_id: Option<String>,
    pub is_active: bool,
}

/// For inserting a new row into the `server_tools` table
#[derive(Debug, Insertable)]
#[diesel(table_name = server_tools)]
pub struct NewServerTool {
    pub id: String,
    pub name: String,
    pub description: String,
    pub input_schema: Option<String>,
    pub server_id: String,
    pub proxy_id: Option<String>,
    pub is_active: bool,
}

/// For updating an existing row in the `server_tools` table
#[derive(Debug, AsChangeset)]
#[diesel(table_name = server_tools)]
#[diesel(primary_key(id, server_id))]
pub struct UpdateServerTool {
    pub name: Option<String>,
    pub description: Option<String>,
    pub input_schema: Option<Option<String>>,
    pub proxy_id: Option<Option<String>>,
    pub is_active: Option<bool>,
}

/// This struct corresponds to a row in the `app_settings` table.
#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = app_settings)]
pub struct DBAppSetting {
    pub key: String,
    pub value: String,
}

/// For inserting a new row into the `app_settings` table
#[derive(Debug, Insertable)]
#[diesel(table_name = app_settings)]
pub struct NewAppSetting<'a> {
    pub key: &'a str,
    pub value: &'a str,
}
