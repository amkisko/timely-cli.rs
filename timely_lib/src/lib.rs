//! Timely API client library.

pub mod api;
pub mod api_http;
pub mod api_session;
pub mod api_templates;
pub mod api_templates_common;
pub mod api_templates_extra;
pub mod api_templates_extra_runtime;
pub mod api_templates_private;
pub mod api_templates_private_runtime;
pub mod api_templates_runtime;
pub mod api_templates_team;
pub mod api_templates_team_runtime;
pub mod auth;
pub mod config;
mod config_keys;
mod config_parse;
pub mod error;
pub mod memory_db;
pub mod oauth;
pub mod openapi;
pub mod runtime_env;
pub mod secrets;
pub mod util;

pub use api::Api;
pub use auth::{StoredCredential, auth_export_value, auth_status_value};
pub use config::{
    ConfigEntry, ConfigSource, config_file_path, ensure_home_config_loaded, friendly_config_key,
    get_config_entry, list_config_entries, resolve_config_key, set_config_entry, timely_home,
    unset_config_entry,
};
pub use error::TimelyError;
pub use oauth::{OAuthExchange, credential_from_token_response, exchange_authorization_code};
pub use secrets::{SecretProvider, SecretSource, fetch_secret};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
