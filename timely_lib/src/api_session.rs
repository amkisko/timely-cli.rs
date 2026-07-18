use std::collections::BTreeMap;

use anyhow::{Context, Result, anyhow};
use reqwest::{Method, StatusCode};
use serde_json::Value;

use crate::api::Api;
use crate::api_http::{finish_response, parse_account_id, parse_numeric_id, read_json_response};
use crate::auth::StoredCredential;
use crate::error::TimelyError;
use crate::oauth::{
    credential_from_token_response, merge_refreshed_credential, oauth_failure_message,
};
use crate::runtime_env;
use crate::util::{fill_path_params, join_url};

impl Api {
    pub async fn send(
        &self,
        method: &str,
        path: &str,
        query: Vec<(String, String)>,
        body: Option<Value>,
    ) -> Result<Value> {
        let method = Method::from_bytes(method.as_bytes()).context("invalid HTTP method")?;
        if let Some(credential) = runtime_env::timely_credential()? {
            let response = self
                .execute_json(
                    method.clone(),
                    path,
                    &query,
                    body.as_ref(),
                    &credential.access_token,
                )
                .await?;
            if response.0 != StatusCode::UNAUTHORIZED {
                return finish_response(response);
            }
            let Some(refreshed) = self.refresh_credential(&credential).await? else {
                return finish_response(response);
            };
            runtime_env::persist_timely_credential(&refreshed)?;
            let retried = self
                .execute_json(method, path, &query, body.as_ref(), &refreshed.access_token)
                .await?;
            return finish_response(retried);
        }

        let credential = self.load_credential()?.ok_or_else(|| {
            TimelyError::Auth(
                "no token configured; run `timely auth token ...` or `timely auth oauth ...`"
                    .to_string(),
            )
        })?;
        let response = self
            .execute_json(
                method.clone(),
                path,
                &query,
                body.as_ref(),
                &credential.access_token,
            )
            .await?;
        if response.0 != StatusCode::UNAUTHORIZED {
            return finish_response(response);
        }

        let Some(refreshed) = self.refresh_credential(&credential).await? else {
            return finish_response(response);
        };
        self.store_credential(refreshed.clone())?;
        let retried = self
            .execute_json(method, path, &query, body.as_ref(), &refreshed.access_token)
            .await?;
        finish_response(retried)
    }

    pub async fn prepare_credential(
        &self,
        mut credential: StoredCredential,
    ) -> Result<StoredCredential> {
        if credential.account_id.is_none()
            && let Ok(account_id) = self.discover_account_id(&credential.access_token).await
        {
            credential.account_id = Some(account_id);
        }
        Ok(credential)
    }

    pub async fn default_account_id(&self) -> Result<i64> {
        if let Some(account_id) = runtime_env::timely_account_id()? {
            return Ok(account_id);
        }
        if let Some(credential) = runtime_env::timely_credential()? {
            let account_id = self
                .discover_account_id(&credential.access_token)
                .await
                .map_err(|err| {
                    anyhow!(
                        "failed to resolve account_id from TIMELY_TOKEN: {err}. {}",
                        "Set TIMELY_ACCOUNT_ID or pass --account-id"
                    )
                })?;
            runtime_env::persist_timely_account_id(account_id)?;
            return Ok(account_id);
        }
        let Some(mut credential) = self.load_credential()? else {
            return Err(TimelyError::Auth(
                "account_id required; pass --account-id, set TIMELY_ACCOUNT_ID, or authenticate first"
                    .to_string(),
            )
            .into());
        };
        if let Some(account_id) = credential.account_id {
            return Ok(account_id);
        }
        let account_id = self.discover_account_id(&credential.access_token).await?;
        credential.account_id = Some(account_id);
        let _ = self.store_credential(credential);
        runtime_env::persist_timely_account_id(account_id)?;
        Ok(account_id)
    }

    pub async fn current_user_id(&self, account_id: Option<i64>) -> Result<i64> {
        let account_id = match account_id {
            Some(account_id) => account_id,
            None => self.default_account_id().await?,
        };
        if let Some(user_id) = self.cached_current_user_id(account_id) {
            return Ok(user_id);
        }
        let path = format!("/1.1/{account_id}/users/current");
        let value = self.send("GET", &path, Vec::new(), None).await?;
        let user_id = parse_numeric_id(&value, "current user")?;
        self.store_cached_current_user_id(account_id, user_id);
        Ok(user_id)
    }

    pub async fn resolve_operation_path(
        &self,
        template: &str,
        mut params: BTreeMap<String, String>,
    ) -> Result<String> {
        if template.contains("{account_id}") && !params.contains_key("account_id") {
            params.insert(
                "account_id".to_string(),
                self.default_account_id().await?.to_string(),
            );
        }
        fill_path_params(template, &params)
    }

    pub async fn resolve_request_path(&self, path: &str) -> Result<String> {
        if !path.contains("{account_id}") {
            return Ok(path.to_string());
        }
        let params = BTreeMap::from([(
            "account_id".to_string(),
            self.default_account_id().await?.to_string(),
        )]);
        fill_path_params(path, &params)
    }

    async fn discover_account_id(&self, access_token: &str) -> Result<i64> {
        let response = self
            .execute_json(Method::GET, "/1.1/accounts", &[], None, access_token)
            .await?;
        let value = finish_response(response)?;
        parse_account_id(&value)
    }

    async fn refresh_credential(
        &self,
        credential: &StoredCredential,
    ) -> Result<Option<StoredCredential>> {
        let Some(refresh_token) = credential.refresh_token.clone() else {
            return Ok(None);
        };
        let Some(client_id) = credential.oauth_client_id.clone() else {
            return Ok(None);
        };
        let token_url = join_url(&self.base_url, "/1.1/oauth/token")?;
        let mut form = vec![
            ("grant_type", "refresh_token".to_string()),
            ("refresh_token", refresh_token),
            ("client_id", client_id),
        ];
        if let Some(secret) = credential.oauth_client_secret.clone() {
            form.push(("client_secret", secret));
        }

        let response = self
            .client
            .post(token_url)
            .form(&form)
            .send()
            .await
            .context("token refresh failed")?;
        let status = response.status();
        let value: Value = response
            .json()
            .await
            .context("invalid token refresh response")?;
        if !status.is_success() {
            return Err(TimelyError::Api(oauth_failure_message(
                "refresh",
                status.as_u16(),
                &value,
            ))
            .into());
        }
        let refreshed = credential_from_token_response(value)?;
        Ok(Some(merge_refreshed_credential(credential, refreshed)))
    }

    async fn execute_json(
        &self,
        method: Method,
        path: &str,
        query: &[(String, String)],
        body: Option<&Value>,
        access_token: &str,
    ) -> Result<(StatusCode, Value)> {
        let url = join_url(&self.base_url, path)?;
        let mut request = self
            .client
            .request(method, url)
            .bearer_auth(access_token)
            .query(query);
        if let Some(body) = body {
            request = request.json(body);
        }

        let response = request.send().await.context("request failed")?;
        read_json_response(response).await
    }
}
