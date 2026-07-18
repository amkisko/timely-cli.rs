use std::fmt;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::api::Api;
use crate::util::{now_epoch, token_fingerprint};

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredCredential {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_type: Option<String>,
    pub scope: Option<String>,
    pub expires_in: Option<i64>,
    pub created_at: i64,
    pub account_id: Option<i64>,
    pub oauth_client_id: Option<String>,
    pub oauth_client_secret: Option<String>,
}

impl fmt::Debug for StoredCredential {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StoredCredential")
            .field("access_token", &token_fingerprint(&self.access_token))
            .field(
                "refresh_token",
                &self
                    .refresh_token
                    .as_ref()
                    .map(|token| token_fingerprint(token)),
            )
            .field("token_type", &self.token_type)
            .field("scope", &self.scope)
            .field("expires_in", &self.expires_in)
            .field("created_at", &self.created_at)
            .field("account_id", &self.account_id)
            .field("oauth_client_id", &self.oauth_client_id)
            .field(
                "oauth_client_secret",
                &self.oauth_client_secret.as_ref().map(|_| "[redacted]"),
            )
            .finish()
    }
}

impl StoredCredential {
    pub fn bearer(access_token: String) -> Self {
        Self {
            access_token,
            refresh_token: None,
            token_type: Some("Bearer".to_string()),
            scope: None,
            expires_in: None,
            created_at: now_epoch(),
            account_id: None,
            oauth_client_id: None,
            oauth_client_secret: None,
        }
    }
}

pub fn auth_status_value(api: &Api) -> Result<Value> {
    let mut value = json!({
        "profile": api.profile,
        "token_configured": false,
        "account_id": null,
        "refresh_token": "not available",
        "scope": null,
    });
    if let Some(credential) = api.load_credential()? {
        value["token_configured"] = Value::Bool(true);
        value["token_fingerprint"] = Value::String(token_fingerprint(&credential.access_token));
        value["account_id"] = credential
            .account_id
            .map(|id| json!(id))
            .unwrap_or(Value::Null);
        value["refresh_token"] = Value::String(if credential.refresh_token.is_some() {
            "available".to_string()
        } else {
            "not available".to_string()
        });
        if let Some(scope) = credential.scope {
            value["scope"] = Value::String(scope);
        }
    }
    Ok(value)
}

/// Redacted local storage and process-cache snapshot for `timely auth export`.
pub fn auth_export_value(api: &Api) -> Result<Value> {
    let cache: Vec<Value> = api
        .cached_current_user_ids()
        .into_iter()
        .map(|(account_id, user_id)| {
            json!({
                "account_id": account_id,
                "user_id": user_id,
            })
        })
        .collect();
    Ok(build_auth_export(
        &api.profile,
        &api.base_url,
        auth_status_value(api)?,
        crate::runtime_env::export_key_presence()?,
        cache,
    ))
}

fn build_auth_export(
    profile: &str,
    base_url: &str,
    keyring: Value,
    runtime_env: Value,
    current_user_ids: Vec<Value>,
) -> Value {
    json!({
        "profile": profile,
        "base_url": base_url,
        "keyring": keyring,
        "runtime_env": runtime_env,
        "process_cache": {
            "current_user_ids": current_user_ids,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bearer_credentials_are_serializable() {
        let credential = StoredCredential::bearer("abc".to_string());
        let encoded = serde_json::to_string(&credential).unwrap();
        let decoded: StoredCredential = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded.access_token, "abc");
        assert_eq!(decoded.token_type.as_deref(), Some("Bearer"));
        assert_eq!(decoded.account_id, None);
    }

    #[test]
    fn credential_debug_redacts_secrets() {
        let mut credential = StoredCredential::bearer("super-secret-token".to_string());
        credential.refresh_token = Some("refresh-secret".to_string());
        credential.oauth_client_secret = Some("client-secret".to_string());
        let rendered = format!("{credential:?}");
        assert!(!rendered.contains("super-secret-token"));
        assert!(!rendered.contains("refresh-secret"));
        assert!(!rendered.contains("client-secret"));
        assert!(rendered.contains("sha256:"));
        assert!(rendered.contains("[redacted]"));
    }

    #[test]
    fn auth_export_payload_uses_fingerprints_not_raw_tokens() {
        let secret = "super-secret-token";
        let keyring = json!({
            "profile": "default",
            "token_configured": true,
            "token_fingerprint": token_fingerprint(secret),
            "account_id": 42,
            "refresh_token": "available",
            "scope": "read",
        });
        let runtime = json!({
            "env_file": null,
            "process": { "TIMELY_TOKEN": true },
            "file": { "TIMELY_TOKEN": false },
        });
        let value = build_auth_export(
            "default",
            "https://api.timelyapp.com",
            keyring,
            runtime,
            vec![json!({"account_id": 42, "user_id": 7})],
        );
        let text = serde_json::to_string(&value).unwrap();
        assert!(!text.contains(secret));
        assert!(!text.contains("access_token"));
        assert!(text.contains("sha256:"));
        assert_eq!(value["keyring"]["refresh_token"], "available");
        assert_eq!(value["process_cache"]["current_user_ids"][0]["user_id"], 7);
        assert_eq!(value["runtime_env"]["process"]["TIMELY_TOKEN"], true);
    }
}
