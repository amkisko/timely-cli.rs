//! Home-directory configuration for non-secret Timely defaults.
//!
//! Reads `$TIMELY_HOME/config.env` and `$TIMELY_HOME/config.local.env` (default
//! `~/.config/timely`, with legacy `~/.timely` fallback). Project `.timely.env`
//! or `.env` in the working directory is also loaded. Process environment
//! variables always win. Tokens and client secrets are not loaded from these
//! files.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Once;

use crate::config_keys::{CONFIG_KEY_DEFS, ConfigKeyDef, resolve_config_key_def};
use crate::config_parse::{remove_config_line, upsert_config_line};

pub use crate::config_keys::{friendly_config_key, resolve_config_key};
pub use crate::config_parse::parse_config_content;

const CONFIG_FILE: &str = "config.env";
const LOCAL_CONFIG_FILE: &str = "config.local.env";

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigSource {
    Env,
    LocalFile,
    File,
    ProjectFile,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ConfigEntry {
    pub key: String,
    pub value: Option<String>,
    pub source: Option<ConfigSource>,
}

/// Resolve the Timely config directory.
///
/// Precedence: `TIMELY_HOME`, then legacy `~/.timely` when it exists and the
/// XDG path does not, otherwise `$XDG_CONFIG_HOME/timely` (default
/// `~/.config/timely`).
pub fn timely_home() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("TIMELY_HOME") {
        let path = path.trim();
        if !path.is_empty() {
            return Some(PathBuf::from(path));
        }
    }

    let home = home_directory()?;
    let xdg_home = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| home.join(".config"));
    let xdg_timely = xdg_home.join("timely");
    let legacy = home.join(".timely");
    if legacy.exists() && !xdg_timely.exists() {
        Some(legacy)
    } else {
        Some(xdg_timely)
    }
}

static HOME_CONFIG_ONCE: Once = Once::new();

/// Path to the main config file (`config.env`).
pub fn config_file_path() -> Result<PathBuf, String> {
    let home =
        timely_home().ok_or_else(|| "could not resolve timely home directory".to_string())?;
    Ok(home.join(CONFIG_FILE))
}

/// List all known config keys with effective values and sources.
pub fn list_config_entries() -> Result<Vec<ConfigEntry>, String> {
    let home =
        timely_home().ok_or_else(|| "could not resolve timely home directory".to_string())?;
    Ok(CONFIG_KEY_DEFS
        .iter()
        .map(|def| entry_for_key(def, &home))
        .collect())
}

/// Read one config key's effective value.
pub fn get_config_entry(input: &str) -> Result<ConfigEntry, String> {
    let home =
        timely_home().ok_or_else(|| "could not resolve timely home directory".to_string())?;
    let def = resolve_config_key_def(input)?;
    Ok(entry_for_key(def, &home))
}

/// Write a config value to `config.env` (creates `TIMELY_HOME` when needed).
pub fn set_config_entry(input: &str, value: &str) -> Result<ConfigEntry, String> {
    let def = resolve_config_key_def(input)?;
    let value = value.trim();
    if value.is_empty() {
        return Err("config value must not be empty".to_string());
    }
    let home =
        timely_home().ok_or_else(|| "could not resolve timely home directory".to_string())?;
    fs::create_dir_all(&home).map_err(|error| format!("create {}: {error}", home.display()))?;
    let path = home.join(CONFIG_FILE);
    let content = if path.is_file() {
        fs::read_to_string(&path).map_err(|error| format!("read {}: {error}", path.display()))?
    } else {
        String::new()
    };
    let next = upsert_config_line(&content, def.env, value);
    fs::write(&path, next).map_err(|error| format!("write {}: {error}", path.display()))?;
    Ok(entry_for_key(def, &home))
}

/// Remove a config value from `config.env`.
pub fn unset_config_entry(input: &str) -> Result<(), String> {
    let def = resolve_config_key_def(input)?;
    let home =
        timely_home().ok_or_else(|| "could not resolve timely home directory".to_string())?;
    let path = home.join(CONFIG_FILE);
    if !path.is_file() {
        return Ok(());
    }
    let content =
        fs::read_to_string(&path).map_err(|error| format!("read {}: {error}", path.display()))?;
    let next = remove_config_line(&content, def.env);
    fs::write(&path, next).map_err(|error| format!("write {}: {error}", path.display()))?;
    Ok(())
}

/// Load home config once per process before clap and auth read env vars.
pub fn ensure_home_config_loaded() {
    HOME_CONFIG_ONCE.call_once(|| {
        let _ = load_home_config();
    });
}

/// Load config files from [`timely_home`] into the process environment.
pub fn load_home_config() -> Result<(), String> {
    let Some(home) = timely_home() else {
        return Ok(());
    };
    load_config_directory(&home)
}

fn load_config_directory(home: &Path) -> Result<(), String> {
    let original_keys: HashSet<String> = std::env::vars().map(|(key, _)| key).collect();
    let mut merged = HashMap::new();

    for file_name in project_config_file_names() {
        if let Some(path) = project_config_path(file_name) {
            if !path.is_file() {
                continue;
            }
            let content = fs::read_to_string(&path)
                .map_err(|error| format!("read {}: {error}", path.display()))?;
            merged.extend(parse_config_content(&content));
        }
    }

    for file_name in [CONFIG_FILE, LOCAL_CONFIG_FILE] {
        let path = home.join(file_name);
        if !path.is_file() {
            continue;
        }
        let content = fs::read_to_string(&path)
            .map_err(|error| format!("read {}: {error}", path.display()))?;
        merged.extend(parse_config_content(&content));
    }

    apply_process_env_defaults(merged, &original_keys);
    Ok(())
}

/// Intent: clap reads `TIMELY_*` from the process environment at parse time.
/// Apply home-config defaults once on the main thread before other threads
/// start. Upgrade path: a clap value source that reads files without set_var.
#[allow(unsafe_code)]
fn apply_process_env_defaults(merged: HashMap<String, String>, original_keys: &HashSet<String>) {
    for (key, value) in merged {
        if original_keys.contains(&key) {
            continue;
        }
        // SAFETY: single-threaded CLI startup before other threads read env.
        unsafe { std::env::set_var(&key, &value) };
    }
}

fn project_config_file_names() -> [&'static str; 2] {
    [".timely.env", ".env"]
}

fn project_config_path(file_name: &str) -> Option<PathBuf> {
    std::env::current_dir()
        .ok()
        .map(|directory| directory.join(file_name))
}

fn entry_for_key(def: &ConfigKeyDef, home: &Path) -> ConfigEntry {
    let (value, source) = effective_value(def.env, home);
    ConfigEntry {
        key: def.friendly.to_string(),
        value,
        source,
    }
}

fn effective_value(env_key: &str, home: &Path) -> (Option<String>, Option<ConfigSource>) {
    if let Ok(value) = std::env::var(env_key) {
        let value = value.trim().to_string();
        if !value.is_empty() {
            return (Some(value), Some(ConfigSource::Env));
        }
    }

    let local_path = home.join(LOCAL_CONFIG_FILE);
    if let Ok(content) = fs::read_to_string(local_path) {
        let entries = parse_config_content(&content);
        if let Some(value) = entries.get(env_key) {
            return (Some(value.clone()), Some(ConfigSource::LocalFile));
        }
    }

    let path = home.join(CONFIG_FILE);
    if let Ok(content) = fs::read_to_string(path) {
        let entries = parse_config_content(&content);
        if let Some(value) = entries.get(env_key) {
            return (Some(value.clone()), Some(ConfigSource::File));
        }
    }

    for file_name in project_config_file_names() {
        if let Some(project_path) = project_config_path(file_name)
            && let Ok(content) = fs::read_to_string(project_path)
        {
            let entries = parse_config_content(&content);
            if let Some(value) = entries.get(env_key) {
                return (Some(value.clone()), Some(ConfigSource::ProjectFile));
            }
        }
    }

    (None, None)
}

fn home_directory() -> Option<PathBuf> {
    for key in ["HOME", "USERPROFILE"] {
        if let Some(path) = std::env::var_os(key) {
            return Some(PathBuf::from(path));
        }
    }
    None
}

#[cfg(test)]
#[path = "config_home_tests.rs"]
mod tests;
