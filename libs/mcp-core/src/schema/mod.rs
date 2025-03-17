// @generated automatically by Diesel CLI.

diesel::table! {
    server_tools (id, server_id) {
        id -> Text,
        name -> Text,
        description -> Text,
        input_schema -> Nullable<Text>,
        server_id -> Text,
        proxy_id -> Nullable<Text>,
        is_active -> Bool,
    }
}

diesel::table! {
    server_env (server_id, env_key) {
        server_id -> Text,
        env_key -> Text,
        env_value -> Text,
        env_description -> Text,
        env_required -> Bool,
    }
}

diesel::table! {
    servers (id) {
        id -> Text,
        name -> Text,
        description -> Text,
        tools_type -> Text,
        enabled -> Bool,
        entry_point -> Nullable<Text>,
        command -> Nullable<Text>,
        args -> Nullable<Text>,
        distribution_type -> Nullable<Text>,
        distribution_package -> Nullable<Text>,
    }
}

diesel::table! {
    app_settings (key) {
        key -> Text,
        value -> Text,
    }
}

diesel::joinable!(server_env -> servers (server_id));

diesel::allow_tables_to_appear_in_same_query!(server_tools, server_env, servers, app_settings,);
