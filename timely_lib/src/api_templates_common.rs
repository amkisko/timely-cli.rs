//! Shared argument helpers for MCP template runtimes.

use std::collections::BTreeMap;

use anyhow::Result;
use serde_json::Value;

use crate::api::Api;

pub async fn account_id(api: &Api, args: &Value) -> Result<i64> {
    match args.get("account_id").and_then(Value::as_i64) {
        Some(account_id) => Ok(account_id),
        None => api.default_account_id().await,
    }
}

pub fn insert_query_value(query: &mut BTreeMap<String, String>, args: &Value, key: &str) {
    if let Some(value) = args.get(key).and_then(stringify_scalar) {
        query.insert(key.to_string(), value);
    }
}

pub fn stringify_scalar(value: &Value) -> Option<String> {
    value
        .as_str()
        .map(str::to_string)
        .or_else(|| value.is_number().then(|| value.to_string()))
        .or_else(|| value.is_boolean().then(|| value.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn insert_query_value_stringifies_scalars() {
        let args = json!({"page": 2, "day": "2026-01-01", "all_users": true});
        let mut query = BTreeMap::new();
        insert_query_value(&mut query, &args, "page");
        insert_query_value(&mut query, &args, "day");
        insert_query_value(&mut query, &args, "all_users");
        assert_eq!(query.get("page").map(String::as_str), Some("2"));
        assert_eq!(query.get("day").map(String::as_str), Some("2026-01-01"));
        assert_eq!(query.get("all_users").map(String::as_str), Some("true"));
    }
}
