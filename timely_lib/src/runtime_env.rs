use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::{Value, json};

use crate::auth::StoredCredential;

const ENV_FILE_VAR: &str = "TIMELY_ENV_FILE";
const DEFAULT_ENV_FILE: &str = ".env";

const EXPORT_KEYS: &[&str] = &[
    "TIMELY_TOKEN",
    "TIMELY_REFRESH_TOKEN",
    "TIMELY_CLIENT_ID",
    "TIMELY_CLIENT_SECRET",
    "TIMELY_ACCOUNT_ID",
    "TIMELY_PROFILE",
    "TIMELY_BASE_URL",
    "TIMELY_MEMORY_DB",
    "TIMELY_ENV_FILE",
    "TIMELY_HOME",
];

pub fn get(key: &str) -> Result<Option<String>> {
    if let Ok(value) = env::var(key) {
        let value = value.trim();
        if !value.is_empty() {
            return Ok(Some(value.to_string()));
        }
    }
    let Some(path) = read_path()? else {
        return Ok(None);
    };
    Ok(read_key(&path, key))
}

pub fn set(key: &str, value: &str) -> Result<Option<String>> {
    let Some(path) = write_path()? else {
        return Ok(None);
    };
    upsert_key(&path, key, value)?;
    Ok(Some(path.display().to_string()))
}

pub fn timely_credential() -> Result<Option<StoredCredential>> {
    let Some(access_token) = get("TIMELY_TOKEN")? else {
        return Ok(None);
    };
    Ok(Some(StoredCredential {
        access_token,
        refresh_token: get("TIMELY_REFRESH_TOKEN")?,
        token_type: Some("Bearer".to_string()),
        scope: None,
        expires_in: None,
        created_at: 0,
        account_id: timely_account_id()?,
        oauth_client_id: get("TIMELY_CLIENT_ID")?,
        oauth_client_secret: get("TIMELY_CLIENT_SECRET")?,
    }))
}

pub fn timely_account_id() -> Result<Option<i64>> {
    let Some(value) = get("TIMELY_ACCOUNT_ID")? else {
        return Ok(None);
    };
    Ok(Some(
        value
            .parse()
            .context("TIMELY_ACCOUNT_ID is not a valid integer")?,
    ))
}

pub fn persist_timely_credential(credential: &StoredCredential) -> Result<()> {
    let _ = set("TIMELY_TOKEN", &credential.access_token)?;
    if let Some(refresh_token) = &credential.refresh_token {
        let _ = set("TIMELY_REFRESH_TOKEN", refresh_token)?;
    }
    if let Some(account_id) = credential.account_id {
        persist_timely_account_id(account_id)?;
    }
    Ok(())
}

pub fn persist_timely_account_id(account_id: i64) -> Result<()> {
    let _ = set("TIMELY_ACCOUNT_ID", &account_id.to_string())?;
    Ok(())
}

pub fn env_file_path() -> Result<Option<String>> {
    Ok(read_path()?.map(|path| path.display().to_string()))
}

/// Presence map for Timely-related keys. Never includes secret values.
pub fn export_key_presence() -> Result<Value> {
    let file_path = read_path()?;
    let mut process = BTreeMap::new();
    let mut file = BTreeMap::new();
    for key in EXPORT_KEYS {
        let in_process = env::var(key)
            .ok()
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false);
        process.insert((*key).to_string(), Value::Bool(in_process));
        let in_file = file_path
            .as_ref()
            .and_then(|path| read_key(path, key))
            .is_some_and(|value| !value.trim().is_empty());
        file.insert((*key).to_string(), Value::Bool(in_file));
    }
    Ok(json!({
        "env_file": file_path.map(|path| path.display().to_string()),
        "process": process,
        "file": file,
    }))
}

fn read_path() -> Result<Option<PathBuf>> {
    if let Some(path) = explicit_path()? {
        return Ok(path.exists().then_some(path));
    }
    let path = env::current_dir()
        .context("failed to read current directory")?
        .join(DEFAULT_ENV_FILE);
    Ok(path.exists().then_some(path))
}

fn write_path() -> Result<Option<PathBuf>> {
    if let Some(path) = explicit_path()? {
        return Ok(Some(path));
    }
    let path = env::current_dir()
        .context("failed to read current directory")?
        .join(DEFAULT_ENV_FILE);
    Ok(path.exists().then_some(path))
}

fn explicit_path() -> Result<Option<PathBuf>> {
    let Some(path) = env::var(ENV_FILE_VAR).ok() else {
        return Ok(None);
    };
    let path = path.trim();
    if path.is_empty() {
        return Ok(None);
    }
    Ok(Some(PathBuf::from(path)))
}

fn read_key(path: &Path, key: &str) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let trimmed = trimmed.strip_prefix("export ").unwrap_or(trimmed);
        let (name, value) = trimmed.split_once('=')?;
        if name.trim() == key {
            return Some(decode_value(value));
        }
    }
    None
}

fn upsert_key(path: &Path, key: &str, value: &str) -> Result<()> {
    let existing = fs::read_to_string(path).unwrap_or_default();
    let mut lines = existing.lines().map(str::to_string).collect::<Vec<_>>();
    let mut found = false;
    for line in &mut lines {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let trimmed = trimmed.strip_prefix("export ").unwrap_or(trimmed);
        let Some((name, _)) = trimmed.split_once('=') else {
            continue;
        };
        if name.trim() == key {
            *line = format!("{key}={}", encode_value(value));
            found = true;
            break;
        }
    }
    if !found {
        lines.push(format!("{key}={}", encode_value(value)));
    }
    let mut out = lines.join("\n");
    if !out.is_empty() {
        out.push('\n');
    }
    fs::write(path, out).with_context(|| format!("failed to write {}", path.display()))
}

fn decode_value(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
        return trimmed[1..trimmed.len() - 1].replace("\\\"", "\"");
    }
    trimmed.to_string()
}

fn encode_value(value: &str) -> String {
    if value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | ':' | '/'))
    {
        return value.to_string();
    }
    format!("\"{}\"", value.replace('"', "\\\""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_plain_and_quoted_values() {
        assert_eq!(decode_value("123"), "123");
        assert_eq!(decode_value("\"abc\""), "abc");
    }

    #[test]
    fn keeps_safe_values_unquoted() {
        assert_eq!(encode_value("abc-123"), "abc-123");
        assert_eq!(encode_value("a b"), "\"a b\"");
    }
}
