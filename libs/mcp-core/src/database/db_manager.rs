use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager, Pool};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use log::info;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use crate::models::tool_db::{DBTool, DBToolEnv, NewTool, NewToolEnv, UpdateTool};
use crate::models::types::{Distribution, Tool, ToolConfiguration, ToolEnvironment};
use crate::schema::server_tools::dsl as server_dsl;
use crate::schema::tool_env::dsl as env_dsl;
use crate::schema::tools::dsl as tools_dsl;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/sqlite");

type SqlitePool = Pool<ConnectionManager<SqliteConnection>>;

#[derive(Clone)]
/// Database manager for persisting application state
pub struct DBManager {
    pool: Arc<SqlitePool>,
}

impl DBManager {
    /// Initialize the database manager with the default database path
    pub fn new() -> Result<Self, String> {
        let storage_path = crate::utils::default_storage_path()?;
        let db_path = storage_path.join("mcp_dockmaster.db");
        Self::with_path(db_path)
    }

    /// Initialize the database manager with a custom database path
    pub fn with_path(db_path: PathBuf) -> Result<Self, String> {
        // Ensure the parent directory exists
        if let Some(parent) = db_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create database directory: {}", e))?;
            }
        }

        // Create the database URL - use file: prefix for SQLite
        let database_url = format!("sqlite://{}", db_path.to_string_lossy());

        // Ensure the database file exists
        if !db_path.exists() {
            std::fs::File::create(&db_path)
                .map_err(|e| format!("Failed to create database file: {}", e))?;
        }

        // Create the connection manager
        let manager = ConnectionManager::<SqliteConnection>::new(database_url);

        // Create the connection pool with more conservative settings
        let pool = r2d2::Pool::builder()
            .max_size(5)
            .connection_timeout(std::time::Duration::from_secs(5))
            .build(manager)
            .map_err(|e| format!("Failed to create connection pool: {}", e))?;

        let db_manager = Self {
            pool: Arc::new(pool),
        };

        info!("Database initialized at: {:?}", db_path);
        Ok(db_manager)
    }

    pub fn apply_migrations(&self) -> Result<(), String> {
        info!("Applying migrations");
        // Run migrations
        let mut conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get database connection: {}", e))?;
        let migration_result = conn.run_pending_migrations(MIGRATIONS);
        match &migration_result {
            Ok(migrations) => {
                info!("Successfully ran {} migrations", migrations.len());
                for migration in migrations {
                    info!("Applied migration: {}", migration);
                }
            }
            Err(e) => {
                info!("Migration failed: {}", e);
            }
        }
        migration_result.map_err(|e| format!("Failed to run migrations: {}", e))?;

        // Debug: Check if tables exist and their structure
        let tables = diesel::sql_query("SELECT name FROM sqlite_master WHERE type='table';")
            .execute(&mut conn)
            .map_err(|e| format!("Failed to check tables: {}", e))?;

        info!("Found {} tables after migrations", tables);

        // Set pragmas
        diesel::sql_query("PRAGMA journal_mode=WAL")
            .execute(&mut conn)
            .map_err(|e| format!("Failed to set journal_mode: {}", e))?;

        diesel::sql_query("PRAGMA synchronous=FULL")
            .execute(&mut conn)
            .map_err(|e| format!("Failed to set synchronous: {}", e))?;

        diesel::sql_query("PRAGMA temp_store=MEMORY")
            .execute(&mut conn)
            .map_err(|e| format!("Failed to set temp_store: {}", e))?;

        diesel::sql_query("PRAGMA optimize")
            .execute(&mut conn)
            .map_err(|e| format!("Failed to optimize: {}", e))?;

        diesel::sql_query("PRAGMA busy_timeout = 5000")
            .execute(&mut conn)
            .map_err(|e| format!("Failed to set busy_timeout: {}", e))?;

        diesel::sql_query("PRAGMA mmap_size=262144000")
            .execute(&mut conn)
            .map_err(|e| format!("Failed to set mmap_size: {}", e))?;

        diesel::sql_query("PRAGMA foreign_keys = ON")
            .execute(&mut conn)
            .map_err(|e| format!("Failed to enable foreign keys: {}", e))?;

        info!("Migrations applied successfully");
        Ok(())
    }

    /// Get a tool by ID
    pub fn get_tool(&self, tool_id_str: &str) -> Result<Tool, String> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get database connection: {}", e))?;

        // 1) Fetch from `tools` table
        let db_tool: DBTool = tools_dsl::tools
            .filter(tools_dsl::id.eq(tool_id_str))
            .first::<DBTool>(&mut conn)
            .map_err(|e| format!("Failed to get tool {}: {}", tool_id_str, e))?;

        // 2) Fetch environment variables from `tool_env` table
        let env_rows: Vec<DBToolEnv> = env_dsl::tool_env
            .filter(env_dsl::tool_id.eq(tool_id_str))
            .load::<DBToolEnv>(&mut conn)
            .map_err(|e| format!("Failed to get env vars for {}: {}", tool_id_str, e))?;

        // Convert environment variables into a HashMap
        let mut env_map = HashMap::new();
        for row in env_rows {
            env_map.insert(
                row.env_key,
                ToolEnvironment {
                    description: row.env_description,
                    default: Some(row.env_value),
                    required: row.env_required,
                },
            );
        }

        // 3) Convert DBTool -> domain-level Tool
        // Parse `args` from DBTool as JSON array
        let parsed_args: Option<Vec<String>> = match db_tool.args {
            Some(ref s) => serde_json::from_str(s).ok(),
            None => None,
        };

        let distribution = if let Some(dist_type) = db_tool.distribution_type.as_ref() {
            Some(Distribution {
                r#type: dist_type.clone(),
                package: db_tool.distribution_package.clone().unwrap_or_default(),
            })
        } else {
            None
        };

        let tool = Tool {
            name: db_tool.name,
            description: db_tool.description,
            enabled: db_tool.enabled,
            tool_type: db_tool.tool_type,
            entry_point: db_tool.entry_point,
            configuration: Some(ToolConfiguration {
                command: db_tool.command,
                args: parsed_args,
                env: if env_map.is_empty() {
                    None
                } else {
                    Some(env_map)
                },
            }),
            distribution,
        };

        Ok(tool)
    }

    /// Get all tools
    pub fn get_all_tools(&self) -> Result<HashMap<String, Tool>, String> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get database connection: {}", e))?;

        // 1) Fetch all tools from the `tools` table
        let db_tools: Vec<DBTool> = tools_dsl::tools
            .load::<DBTool>(&mut conn)
            .map_err(|e| format!("Failed to query tools: {}", e))?;

        // 2) Fetch all environment variables
        let all_env_rows: Vec<DBToolEnv> = env_dsl::tool_env
            .load::<DBToolEnv>(&mut conn)
            .map_err(|e| format!("Failed to query environment variables: {}", e))?;

        // Group environment variables by tool_id
        let mut env_map_by_tool: HashMap<String, HashMap<String, ToolEnvironment>> = HashMap::new();
        for row in all_env_rows {
            let tool_env_map = env_map_by_tool.entry(row.tool_id.clone()).or_default();
            tool_env_map.insert(
                row.env_key.clone(),
                ToolEnvironment {
                    description: row.env_description,
                    default: Some(row.env_value),
                    required: row.env_required,
                },
            );
        }

        // 3) Convert DBTool -> domain-level Tool for each tool
        let mut tools_map = HashMap::new();
        for db_tool in db_tools {
            // Parse `args` from DBTool as JSON array
            let parsed_args: Option<Vec<String>> = match db_tool.args {
                Some(ref s) => serde_json::from_str(s).ok(),
                None => None,
            };

            let distribution = if let Some(dist_type) = db_tool.distribution_type.as_ref() {
                Some(Distribution {
                    r#type: dist_type.clone(),
                    package: db_tool.distribution_package.clone().unwrap_or_default(),
                })
            } else {
                None
            };

            // Get environment variables for this tool
            let env_map = env_map_by_tool.remove(&db_tool.id).unwrap_or_default();

            let tool = Tool {
                name: db_tool.name.clone(),
                description: db_tool.description.clone(),
                enabled: db_tool.enabled,
                tool_type: db_tool.tool_type.clone(),
                entry_point: db_tool.entry_point.clone(),
                configuration: Some(ToolConfiguration {
                    command: db_tool.command.clone(),
                    args: parsed_args,
                    env: if env_map.is_empty() {
                        None
                    } else {
                        Some(env_map)
                    },
                }),
                distribution,
            };

            tools_map.insert(db_tool.id.clone(), tool);
        }

        Ok(tools_map)
    }

    /// Save or update a tool
    pub fn save_tool(&self, tool_id_str: &str, tool: &Tool) -> Result<(), String> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get database connection: {}", e))?;

        // Convert domain `Tool` into row data
        let distribution_type_str = tool.distribution.as_ref().map(|d| d.r#type.clone());
        let distribution_package_str = tool.distribution.as_ref().map(|d| d.package.clone());

        // Store the `args` as JSON in a text column
        let args_as_str = if let Some(config) = &tool.configuration {
            config
                .args
                .as_ref()
                .map(|args_vec| serde_json::to_string(args_vec).unwrap_or_default())
        } else {
            None
        };

        let command_str = tool
            .configuration
            .as_ref()
            .and_then(|c| c.command.clone())
            .unwrap_or_default();

        // Prepare upsert struct
        let new_tool = NewTool {
            id: tool_id_str,
            name: &tool.name,
            description: &tool.description,
            tool_type: &tool.tool_type,
            enabled: tool.enabled,
            entry_point: tool.entry_point.as_deref(),
            command: if command_str.is_empty() {
                None
            } else {
                Some(&command_str)
            },
            args: args_as_str.as_deref(),
            distribution_type: distribution_type_str.as_deref(),
            distribution_package: distribution_package_str.as_deref(),
        };

        // For updates, we need to create an UpdateTool struct
        let update_tool = UpdateTool {
            name: Some(&tool.name),
            description: Some(&tool.description),
            tool_type: Some(&tool.tool_type),
            enabled: Some(tool.enabled),
            entry_point: Some(tool.entry_point.as_deref()),
            command: Some(if command_str.is_empty() {
                None
            } else {
                Some(&command_str)
            }),
            args: Some(args_as_str.as_deref()),
            distribution_type: Some(distribution_type_str.as_deref()),
            distribution_package: Some(distribution_package_str.as_deref()),
        };

        // Insert or update main row
        diesel::insert_into(tools_dsl::tools)
            .values(&new_tool)
            .on_conflict(tools_dsl::id)
            .do_update()
            .set(&update_tool)
            .execute(&mut conn)
            .map_err(|e| format!("Failed to save tool: {}", e))?;

        // Now handle environment variables in tool_env
        // 1) Delete old environment variables
        diesel::delete(env_dsl::tool_env.filter(env_dsl::tool_id.eq(tool_id_str)))
            .execute(&mut conn)
            .map_err(|e| format!("Failed to clear old env: {}", e))?;

        // 2) Insert new environment variables
        if let Some(config) = &tool.configuration {
            if let Some(env) = &config.env {
                let new_env_rows: Vec<NewToolEnv> = env
                    .iter()
                    .map(|(k, v)| {
                        let default_value = v.default.clone().unwrap_or_default();
                        NewToolEnv {
                            tool_id: tool_id_str.to_string(),
                            env_key: k.to_string(),
                            env_value: default_value,
                            env_description: v.description.clone(),
                            env_required: v.required,
                        }
                    })
                    .collect();

                if !new_env_rows.is_empty() {
                    diesel::insert_into(env_dsl::tool_env)
                        .values(&new_env_rows)
                        .execute(&mut conn)
                        .map_err(|e| format!("Failed to save env vars: {}", e))?;
                }
            }
        }

        Ok(())
    }

    /// Delete a tool by ID
    pub fn delete_tool(&self, tool_id_str: &str) -> Result<(), String> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get database connection: {}", e))?;

        // Delete environment variables first (foreign key constraint will ensure this happens)
        diesel::delete(env_dsl::tool_env.filter(env_dsl::tool_id.eq(tool_id_str)))
            .execute(&mut conn)
            .map_err(|e| format!("Failed to delete tool environment variables: {}", e))?;

        // Delete the tool
        diesel::delete(tools_dsl::tools.filter(tools_dsl::id.eq(tool_id_str)))
            .execute(&mut conn)
            .map_err(|e| format!("Failed to delete tool: {}", e))?;

        Ok(())
    }

    /// Clear the database
    pub fn clear_database(&mut self) -> Result<(), String> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get database connection: {}", e))?;

        conn.transaction::<_, diesel::result::Error, _>(|conn| {
            // Delete environment variables first (due to foreign key constraints)
            diesel::delete(env_dsl::tool_env).execute(conn)?;

            // Delete tools
            diesel::delete(tools_dsl::tools).execute(conn)?;

            // Delete server tools
            diesel::delete(server_dsl::server_tools).execute(conn)?;

            Ok(())
        })
        .map_err(|e| format!("Transaction failed: {}", e))?;

        info!("Database cleared successfully");
        Ok(())
    }

    /// Check if the database exists and has data
    pub fn check_exists(&self) -> Result<bool, String> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get database connection: {}", e))?;

        let count: i64 = tools_dsl::tools
            .count()
            .get_result(&mut conn)
            .map_err(|e| format!("Failed to check database: {}", e))?;

        Ok(count > 0)
    }

    /// Safely close the database connection
    pub fn close(self) -> Result<(), String> {
        // The connection pool will be dropped when self is dropped
        // No explicit handling needed
        info!("Database connection closed");
        Ok(())
    }
}
