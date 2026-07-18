use std::fmt;

use anyhow::{Result, anyhow};
use serde_json::Value;

use crate::api::Api;
use crate::auth::StoredCredential;
use crate::util::{MAX_ERROR_BODY_CHARS, join_url, now_epoch, truncate_for_display};

#[derive(Clone)]
pub struct OAuthExchange {
    pub client_id: String,
    pub client_secret: Option<String>,
    pub redirect_uri: String,
    pub code: String,
    pub verifier: String,
}

impl fmt::Debug for OAuthExchange {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OAuthExchange")
            .field("client_id", &self.client_id)
            .field(
                "client_secret",
                &self.client_secret.as_ref().map(|_| "[redacted]"),
            )
            .field("redirect_uri", &self.redirect_uri)
            .field("code", &"[redacted]")
            .field("verifier", &"[redacted]")
            .finish()
    }
}

pub async fn exchange_authorization_code(
    api: &Api,
    exchange: &OAuthExchange,
) -> Result<StoredCredential> {
    let token_url = join_url(&api.base_url, "/1.1/oauth/token")?;
    let mut form = vec![
        ("grant_type", "authorization_code".to_string()),
        ("code", exchange.code.clone()),
        ("redirect_uri", exchange.redirect_uri.clone()),
        ("client_id", exchange.client_id.clone()),
        ("code_verifier", exchange.verifier.clone()),
    ];
    if let Some(secret) = exchange.client_secret.clone() {
        form.push(("client_secret", secret));
    }

    let response = api.client.post(token_url).form(&form).send().await?;
    let status = response.status();
    let value: Value = response.json().await?;
    if !status.is_success() {
        return Err(crate::error::TimelyError::Api(oauth_failure_message(
            "exchange",
            status.as_u16(),
            &value,
        ))
        .into());
    }
    credential_from_token_response(value)
}

pub fn oauth_failure_message(operation: &str, status: u16, value: &Value) -> String {
    let error_code = value
        .get("error")
        .and_then(Value::as_str)
        .unwrap_or("request_failed");
    let description = value
        .get("error_description")
        .and_then(Value::as_str)
        .unwrap_or("");
    if description.is_empty() {
        format!("OAuth token {operation} failed with {status}: {error_code}")
    } else {
        format!(
            "OAuth token {operation} failed with {status}: {error_code}: {}",
            truncate_for_display(description, MAX_ERROR_BODY_CHARS)
        )
    }
}

pub fn credential_from_token_response(value: Value) -> Result<StoredCredential> {
    let access_token = value
        .get("access_token")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("OAuth response did not include access_token"))?
        .to_string();
    Ok(StoredCredential {
        access_token,
        refresh_token: value
            .get("refresh_token")
            .and_then(Value::as_str)
            .map(str::to_string),
        token_type: value
            .get("token_type")
            .and_then(Value::as_str)
            .map(str::to_string),
        scope: value
            .get("scope")
            .and_then(Value::as_str)
            .map(str::to_string),
        expires_in: value.get("expires_in").and_then(Value::as_i64),
        created_at: value
            .get("created_at")
            .and_then(Value::as_i64)
            .unwrap_or_else(now_epoch),
        account_id: None,
        oauth_client_id: None,
        oauth_client_secret: None,
    })
}

pub fn merge_refreshed_credential(
    previous: &StoredCredential,
    mut refreshed: StoredCredential,
) -> StoredCredential {
    refreshed.account_id = previous.account_id;
    refreshed.oauth_client_id = previous.oauth_client_id.clone();
    refreshed.oauth_client_secret = previous.oauth_client_secret.clone();
    if refreshed.refresh_token.is_none() {
        refreshed.refresh_token = previous.refresh_token.clone();
    }
    refreshed
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn merge_keeps_prior_refresh_token_when_response_omits_it() {
        let previous = StoredCredential {
            access_token: "old-access".to_string(),
            refresh_token: Some("keep-me".to_string()),
            token_type: Some("Bearer".to_string()),
            scope: Some("manage".to_string()),
            expires_in: Some(3600),
            created_at: 1,
            account_id: Some(9),
            oauth_client_id: Some("client".to_string()),
            oauth_client_secret: Some("secret".to_string()),
        };
        let refreshed = credential_from_token_response(json!({
            "access_token": "new-access",
            "token_type": "Bearer",
            "created_at": 2
        }))
        .unwrap();

        let merged = merge_refreshed_credential(&previous, refreshed);

        assert_eq!(merged.access_token, "new-access");
        assert_eq!(merged.refresh_token.as_deref(), Some("keep-me"));
        assert_eq!(merged.account_id, Some(9));
        assert_eq!(merged.oauth_client_id.as_deref(), Some("client"));
        assert_eq!(merged.oauth_client_secret.as_deref(), Some("secret"));
    }

    #[test]
    fn merge_prefers_rotated_refresh_token() {
        let previous = StoredCredential {
            access_token: "old-access".to_string(),
            refresh_token: Some("old-refresh".to_string()),
            token_type: None,
            scope: None,
            expires_in: None,
            created_at: 1,
            account_id: None,
            oauth_client_id: Some("client".to_string()),
            oauth_client_secret: None,
        };
        let refreshed = credential_from_token_response(json!({
            "access_token": "new-access",
            "refresh_token": "new-refresh"
        }))
        .unwrap();

        let merged = merge_refreshed_credential(&previous, refreshed);

        assert_eq!(merged.refresh_token.as_deref(), Some("new-refresh"));
    }

    #[test]
    fn oauth_failure_message_avoids_raw_token_payload() {
        let message = oauth_failure_message(
            "exchange",
            400,
            &json!({
                "error": "invalid_grant",
                "error_description": "code rejected",
                "access_token": "should-not-appear"
            }),
        );
        assert!(message.contains("invalid_grant"));
        assert!(message.contains("code rejected"));
        assert!(!message.contains("should-not-appear"));
        assert!(!message.contains("access_token"));
    }

    #[test]
    fn oauth_exchange_debug_redacts_secrets() {
        let exchange = OAuthExchange {
            client_id: "client".to_string(),
            client_secret: Some("super-secret".to_string()),
            redirect_uri: "http://127.0.0.1/callback".to_string(),
            code: "auth-code-value".to_string(),
            verifier: "pkce-verifier-value".to_string(),
        };
        let rendered = format!("{exchange:?}");
        assert!(rendered.contains("[redacted]"));
        assert!(!rendered.contains("super-secret"));
        assert!(!rendered.contains("auth-code-value"));
        assert!(!rendered.contains("pkce-verifier-value"));
    }
}
