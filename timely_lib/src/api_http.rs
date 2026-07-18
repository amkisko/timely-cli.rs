//! HTTP response decoding and Timely JSON helpers for Api.

use anyhow::{Context, Result, anyhow};
use reqwest::{Response, StatusCode};
use serde_json::{Value, json};

use crate::error::TimelyError;
use crate::util::{MAX_ERROR_BODY_CHARS, MAX_RESPONSE_BODY_BYTES, truncate_for_display};

pub async fn read_json_response(response: Response) -> Result<(StatusCode, Value)> {
    let status = response.status();
    if let Some(length) = response.content_length()
        && length > MAX_RESPONSE_BODY_BYTES as u64
    {
        return Err(TimelyError::Api(format!(
            "Timely API response exceeded {} bytes",
            MAX_RESPONSE_BODY_BYTES
        ))
        .into());
    }
    let bytes = response
        .bytes()
        .await
        .context("failed to read response body")?;
    if bytes.len() > MAX_RESPONSE_BODY_BYTES {
        return Err(TimelyError::Api(format!(
            "Timely API response exceeded {} bytes",
            MAX_RESPONSE_BODY_BYTES
        ))
        .into());
    }
    let text = String::from_utf8_lossy(&bytes);
    let parsed =
        serde_json::from_str::<Value>(&text).unwrap_or_else(|_| json!({ "body": text.as_ref() }));
    Ok((status, parsed))
}

pub fn finish_response((status, parsed): (StatusCode, Value)) -> Result<Value> {
    if !status.is_success() {
        let body = serde_json::to_string_pretty(&parsed)?;
        return Err(TimelyError::Api(format!(
            "Timely API returned {status}: {}",
            truncate_for_display(&body, MAX_ERROR_BODY_CHARS)
        ))
        .into());
    }
    Ok(parsed)
}

pub fn parse_account_id(value: &Value) -> Result<i64> {
    let first = value
        .as_array()
        .and_then(|accounts| accounts.first())
        .ok_or_else(|| anyhow!("Timely did not return any accounts for this token"))?;
    parse_numeric_id(first, "account")
}

pub fn parse_numeric_id(value: &Value, resource: &str) -> Result<i64> {
    value
        .get("id")
        .and_then(Value::as_i64)
        .ok_or_else(|| anyhow!("Timely {resource} response did not include a numeric id"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_first_account_id() {
        let value = json!([{"id": 42}, {"id": 7}]);
        assert_eq!(parse_account_id(&value).unwrap(), 42);
    }

    #[test]
    fn parses_numeric_id_field() {
        let value = json!({"id": 9});
        assert_eq!(parse_numeric_id(&value, "user").unwrap(), 9);
    }
}
