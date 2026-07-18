use anyhow::{Result, anyhow};
use serde_json::{Value, json};

use crate::api::Api;
use crate::api_templates_common::{account_id, insert_query_value};
use crate::util::value_to_string_map;

pub async fn maybe_call(api: &Api, name: &str, args: &Value) -> Option<Result<Value>> {
    Some(match name {
        "timely_experimental_memory_accounts" => private_get(api, "/1.1/accounts.json").await,
        "timely_experimental_memory_identity" => private_get(api, "/1.1/identity.json").await,
        "timely_experimental_memory_request" => private_request(api, args).await,
        "timely_experimental_memory_linked_entries" => linked_entries(api, args).await,
        _ => return None,
    })
}

async fn private_get(api: &Api, path: &str) -> Result<Value> {
    api.send("GET", path, Vec::new(), None).await
}

async fn private_request(api: &Api, args: &Value) -> Result<Value> {
    let method = args
        .get("method")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("method is required"))?;
    let path = args
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("path is required"))?;
    api.send(
        method,
        path,
        value_to_string_map(args.get("query")).into_iter().collect(),
        args.get("body").cloned(),
    )
    .await
}

async fn linked_entries(api: &Api, args: &Value) -> Result<Value> {
    let account_id = account_id(api, args).await?;
    let mut query = value_to_string_map(args.get("query"));
    for key in [
        "since",
        "upto",
        "day",
        "project_id",
        "user_id",
        "per_page",
        "page",
    ] {
        insert_query_value(&mut query, args, key);
    }
    let all_users = args
        .get("all_users")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !all_users && !query.contains_key("user_id") {
        query.insert(
            "user_id".to_string(),
            api.current_user_id(Some(account_id)).await?.to_string(),
        );
    }
    let page = query.get("page").cloned();
    let per_page = query.get("per_page").cloned();
    let hours = api
        .send(
            "GET",
            &format!("/1.1/{account_id}/hours"),
            query.into_iter().collect(),
            None,
        )
        .await?;
    Ok(extract_linked_entries(hours, page, per_page))
}

fn extract_linked_entries(hours: Value, page: Option<String>, per_page: Option<String>) -> Value {
    let entries = hours
        .as_array()
        .into_iter()
        .flatten()
        .map(|hour| {
            json!({
                "hour_id": hour.get("id").and_then(Value::as_i64),
                "day": hour.get("day"),
                "suggestion_id": hour.get("suggestion_id").and_then(Value::as_i64),
                "entry_ids": int_array(hour.get("entry_ids")),
                "timestamps": hour
                    .get("timestamps")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .map(|timestamp| {
                        json!({
                            "id": timestamp.get("id").and_then(Value::as_i64),
                            "from": timestamp.get("from"),
                            "to": timestamp.get("to"),
                            "entry_ids": int_array(timestamp.get("entry_ids")),
                        })
                    })
                    .collect::<Vec<_>>(),
            })
        })
        .collect::<Vec<_>>();
    let mut result = json!({ "hours": entries });
    if let Some(page) = page {
        result
            .as_object_mut()
            .expect("object")
            .insert("page".to_string(), json!(page));
    }
    if let Some(per_page) = per_page {
        result
            .as_object_mut()
            .expect("object")
            .insert("per_page".to_string(), json!(per_page));
    }
    result
}

fn int_array(value: Option<&Value>) -> Vec<i64> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_i64)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_linked_entries_includes_page_metadata() {
        let value = extract_linked_entries(
            json!([{
                "id": 9,
                "day": "2026-01-01",
                "suggestion_id": 3,
                "entry_ids": [1, 2],
                "timestamps": []
            }]),
            Some("2".to_string()),
            Some("50".to_string()),
        );
        assert_eq!(value["page"], json!("2"));
        assert_eq!(value["per_page"], json!("50"));
        assert_eq!(value["hours"][0]["hour_id"], json!(9));
    }
}
