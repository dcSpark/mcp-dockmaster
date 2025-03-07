CREATE TABLE tools (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    tool_type TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    entry_point TEXT,
    command TEXT,
    args TEXT,
    distribution_type TEXT,
    distribution_package TEXT
);

CREATE TABLE tool_env (
    tool_id TEXT NOT NULL,
    env_key TEXT NOT NULL,
    env_value TEXT NOT NULL,
    env_description TEXT NOT NULL,
    env_required BOOLEAN NOT NULL DEFAULT FALSE,
    PRIMARY KEY (tool_id, env_key),
    FOREIGN KEY (tool_id) REFERENCES tools(id) ON DELETE CASCADE
);

CREATE TABLE server_tools (
    server_id TEXT PRIMARY KEY,
    tool_data TEXT NOT NULL
);
