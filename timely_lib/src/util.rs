use std::collections::BTreeMap;
use std::io::Read;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Result, anyhow, bail};

use crate::error::TimelyError;
use base64::Engine;
use serde_json::Value;
use sha2::{Digest, Sha256};
use url::Url;

pub fn parse_pairs(values: Vec<String>) -> Result<BTreeMap<String, String>> {
    let mut map = BTreeMap::new();
    for value in values {
        let (key, value) = value
            .split_once('=')
            .ok_or_else(|| anyhow!("expected key=value, got '{value}'"))?;
        map.insert(key.to_string(), value.to_string());
    }
    Ok(map)
}

pub fn read_body(body: Option<String>, body_file: Option<String>) -> Result<Option<Value>> {
    match (body, body_file) {
        (Some(_), Some(_)) => bail!("use --body or --body-file, not both"),
        (Some(body), None) => Ok(Some(serde_json::from_str(&body)?)),
        (None, Some(path)) => {
            let text = std::fs::read_to_string(path)?;
            Ok(Some(serde_json::from_str(&text)?))
        }
        (None, None) => Ok(None),
    }
}

pub fn fill_path_params(path: &str, params: &BTreeMap<String, String>) -> Result<String> {
    let mut out = path.to_string();
    for (key, value) in params {
        out = out.replace(&format!("{{{key}}}"), value);
    }
    if out.contains('{') {
        bail!("missing required path parameter for '{out}'");
    }
    Ok(out)
}

pub fn object_to_pairs(value: Option<&Value>) -> Vec<(String, String)> {
    value
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .map(|(key, value)| (key.clone(), scalar_to_string(value)))
        .collect()
}

pub fn value_to_string_map(value: Option<&Value>) -> BTreeMap<String, String> {
    value
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .map(|(key, value)| (key.clone(), scalar_to_string(value)))
        .collect()
}

pub fn join_url(base_url: &str, path: &str) -> Result<String> {
    if path.starts_with("http://") || path.starts_with("https://") {
        return Err(TimelyError::Usage(
            "API path must be relative to the configured base URL".to_string(),
        )
        .into());
    }
    let base = Url::parse(&format!("{}/", base_url.trim_end_matches('/')))?;
    Ok(base.join(path.trim_start_matches('/'))?.to_string())
}

pub fn token_fingerprint(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest);
    format!("sha256:{}", &encoded[..12])
}

/// Read a secret from an inline value or a file path (`-` means stdin).
/// Trims a single trailing newline. Never includes the secret in error text.
pub fn read_secret_value(inline: Option<String>, file: Option<String>) -> Result<String> {
    match (inline, file) {
        (Some(_), Some(_)) => bail!("use an inline secret flag or a file flag, not both"),
        (None, None) => bail!("provide a secret value or a secret file"),
        (Some(value), None) => normalize_secret(&value),
        (None, Some(path)) => {
            let text = if path == "-" {
                let mut buffer = String::new();
                std::io::stdin().read_to_string(&mut buffer)?;
                buffer
            } else {
                std::fs::read_to_string(&path).map_err(|_| anyhow!("failed to read secret file"))?
            };
            normalize_secret(&text)
        }
    }
}

fn normalize_secret(value: &str) -> Result<String> {
    let trimmed = value.trim_end_matches(['\r', '\n']);
    if trimmed.is_empty() {
        bail!("secret value is empty");
    }
    Ok(trimmed.to_string())
}

pub fn now_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

pub const MAX_RESPONSE_BODY_BYTES: usize = 10 * 1024 * 1024;
pub const MAX_ERROR_BODY_CHARS: usize = 500;

pub fn truncate_for_display(text: &str, max_chars: usize) -> String {
    let count = text.chars().count();
    if count <= max_chars {
        return text.to_string();
    }
    let end = text
        .char_indices()
        .nth(max_chars)
        .map(|(index, _)| index)
        .unwrap_or(text.len());
    format!("{}…", &text[..end])
}

fn scalar_to_string(value: &Value) -> String {
    value
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn fills_path_params() {
        let params = BTreeMap::from([("account_id".to_string(), "123".to_string())]);
        assert_eq!(
            fill_path_params("/1.1/{account_id}/projects", &params).unwrap(),
            "/1.1/123/projects"
        );
    }

    #[test]
    fn rejects_missing_path_params() {
        let err = fill_path_params("/1.1/{account_id}/projects", &BTreeMap::new()).unwrap_err();
        assert!(err.to_string().contains("missing required path parameter"));
    }

    #[test]
    fn converts_object_to_query_pairs() {
        let value = json!({"page": 1, "search": "client"});
        assert_eq!(
            object_to_pairs(Some(&value)),
            vec![
                ("page".to_string(), "1".to_string()),
                ("search".to_string(), "client".to_string())
            ]
        );
    }

    #[test]
    fn join_url_builds_relative_paths_against_base() {
        assert_eq!(
            join_url("https://api.timelyapp.com", "/1.1/accounts").unwrap(),
            "https://api.timelyapp.com/1.1/accounts"
        );
    }

    #[test]
    fn join_url_rejects_absolute_http_paths() {
        let error = join_url(
            "https://api.timelyapp.com",
            "https://evil.example/exfiltrate",
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("relative"));
    }

    #[test]
    fn join_url_rejects_absolute_https_paths() {
        let error = join_url(
            "https://api.timelyapp.com",
            "http://evil.example/exfiltrate",
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("relative"));
    }

    #[test]
    fn read_secret_value_from_inline() {
        assert_eq!(
            read_secret_value(Some("secret\n".to_string()), None).unwrap(),
            "secret"
        );
    }

    #[test]
    fn read_secret_value_rejects_empty() {
        let error = read_secret_value(Some("\n".to_string()), None)
            .unwrap_err()
            .to_string();
        assert!(error.contains("empty"));
    }

    #[test]
    fn read_secret_value_rejects_both_sources() {
        let error = read_secret_value(Some("a".to_string()), Some("b".to_string()))
            .unwrap_err()
            .to_string();
        assert!(error.contains("not both"));
    }

    #[test]
    fn read_secret_value_from_file() {
        let path = std::env::temp_dir().join(format!("timely-secret-test-{}", std::process::id()));
        std::fs::write(&path, "file-secret\n").unwrap();
        let value = read_secret_value(None, Some(path.to_string_lossy().into_owned())).unwrap();
        let _ = std::fs::remove_file(&path);
        assert_eq!(value, "file-secret");
    }

    #[test]
    fn truncate_for_display_keeps_short_text() {
        assert_eq!(truncate_for_display("short", 10), "short");
    }

    #[test]
    fn truncate_for_display_limits_long_text() {
        assert_eq!(truncate_for_display("abcdefghij", 4), "abcd…");
    }
}
