use anyhow::{Result, anyhow, bail};
use serde_json::{Map, Value, json};

use crate::api::Api;
use crate::api_templates_common::{account_id, insert_query_value};
use crate::api_templates_extra_runtime;
use crate::api_templates_private_runtime;
use crate::api_templates_team_runtime;
use crate::util::value_to_string_map;

pub async fn call(api: &Api, name: &str, args: &Value) -> Result<Value> {
    maybe_call(api, name, args)
        .await
        .unwrap_or_else(|| Err(anyhow!("unknown api command '{name}'")))
}

pub async fn maybe_call(api: &Api, name: &str, args: &Value) -> Option<Result<Value>> {
    if let Some(result) = api_templates_team_runtime::maybe_call(api, name, args).await {
        return Some(result);
    }
    if let Some(result) = api_templates_extra_runtime::maybe_call(api, name, args).await {
        return Some(result);
    }
    if let Some(result) = api_templates_private_runtime::maybe_call(api, name, args).await {
        return Some(result);
    }
    Some(match name {
        "timely_me" => get_me(api, args).await,
        "timely_list_clients" => list_resource(api, args, "/clients").await,
        "timely_get_client" => get_resource(api, args, "client_id", "/clients").await,
        "timely_list_projects" => list_resource(api, args, "/projects").await,
        "timely_get_project" => get_resource(api, args, "project_id", "/projects").await,
        "timely_list_users" => list_resource(api, args, "/users").await,
        "timely_get_user" => get_resource(api, args, "user_id", "/users").await,
        "timely_list_time_entries" => list_time_entries(api, args).await,
        "timely_get_time_entry" => get_resource(api, args, "time_entry_id", "/hours").await,
        "timely_create_time_entry" => create_time_entry(api, args).await,
        "timely_update_time_entry" => update_time_entry(api, args).await,
        "timely_delete_time_entry" => delete_time_entry(api, args).await,
        "timely_start_timer" => timer_action(api, args, "start").await,
        "timely_stop_timer" => timer_action(api, args, "stop").await,
        "timely_list_labels" => list_resource(api, args, "/labels").await,
        "timely_get_label" => get_resource(api, args, "label_id", "/labels").await,
        "timely_list_tasks" => list_resource(api, args, "/forecasts").await,
        "timely_get_task" => get_resource(api, args, "task_id", "/forecasts").await,
        _ => return None,
    })
}

async fn get_me(api: &Api, args: &Value) -> Result<Value> {
    let account_id = account_id(api, args).await?;
    api.send(
        "GET",
        &format!("/1.1/{account_id}/users/current"),
        Vec::new(),
        None,
    )
    .await
}

async fn list_resource(api: &Api, args: &Value, path: &str) -> Result<Value> {
    let account_id = account_id(api, args).await?;
    let query = value_to_string_map(args.get("query"));
    api.send(
        "GET",
        &format!("/1.1/{account_id}{path}"),
        query.into_iter().collect(),
        None,
    )
    .await
}

async fn get_resource(api: &Api, args: &Value, id_key: &str, path: &str) -> Result<Value> {
    let account_id = account_id(api, args).await?;
    let id = required_i64(args, id_key)?;
    api.send(
        "GET",
        &format!("/1.1/{account_id}{path}/{id}"),
        Vec::new(),
        None,
    )
    .await
}

async fn list_time_entries(api: &Api, args: &Value) -> Result<Value> {
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
        "sort",
        "order",
        "include_linked_metadata",
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
    api.send(
        "GET",
        &format!("/1.1/{account_id}/hours"),
        query.into_iter().collect(),
        None,
    )
    .await
}

async fn create_time_entry(api: &Api, args: &Value) -> Result<Value> {
    let account_id = account_id(api, args).await?;
    let mut hours = Map::new();
    hours.insert(
        "project_id".to_string(),
        json!(required_i64(args, "project_id")?),
    );
    hours.insert("day".to_string(), json!(required_string(args, "day")?));
    copy_fields(
        &mut hours,
        args,
        &[
            "hours",
            "minutes",
            "seconds",
            "note",
            "from",
            "to",
            "label_ids",
        ],
    );
    require_duration(&hours)?;
    api.send(
        "POST",
        &format!("/1.1/{account_id}/hours"),
        Vec::new(),
        Some(json!({ "hours": hours })),
    )
    .await
}

async fn update_time_entry(api: &Api, args: &Value) -> Result<Value> {
    let account_id = account_id(api, args).await?;
    let time_entry_id = required_i64(args, "time_entry_id")?;
    let mut hours = Map::new();
    copy_fields(
        &mut hours,
        args,
        &[
            "project_id",
            "day",
            "hours",
            "minutes",
            "seconds",
            "note",
            "from",
            "to",
            "label_ids",
        ],
    );
    if hours.is_empty() {
        bail!("timely_update_time_entry requires at least one field to update");
    }
    api.send(
        "PUT",
        &format!("/1.1/{account_id}/hours/{time_entry_id}"),
        Vec::new(),
        Some(json!({ "hours": hours })),
    )
    .await
}

async fn delete_time_entry(api: &Api, args: &Value) -> Result<Value> {
    let account_id = account_id(api, args).await?;
    let time_entry_id = required_i64(args, "time_entry_id")?;
    api.send(
        "DELETE",
        &format!("/1.1/{account_id}/hours/{time_entry_id}"),
        Vec::new(),
        None,
    )
    .await
}

async fn timer_action(api: &Api, args: &Value, action: &str) -> Result<Value> {
    let account_id = account_id(api, args).await?;
    let time_entry_id = required_i64(args, "time_entry_id")?;
    api.send(
        "PUT",
        &format!("/1.1/{account_id}/hours/{time_entry_id}/{action}"),
        Vec::new(),
        None,
    )
    .await
}

fn copy_fields(body: &mut Map<String, Value>, args: &Value, fields: &[&str]) {
    for key in fields {
        if let Some(value) = args.get(*key) {
            body.insert((*key).to_string(), value.clone());
        }
    }
}

fn require_duration(hours: &Map<String, Value>) -> Result<()> {
    let has_numeric = ["hours", "minutes", "seconds"]
        .into_iter()
        .any(|key| hours.contains_key(key));
    let has_range = hours.contains_key("from") && hours.contains_key("to");
    if has_numeric || has_range {
        return Ok(());
    }
    bail!("time entries need hours/minutes/seconds or both from and to")
}

fn required_i64(args: &Value, key: &str) -> Result<i64> {
    args.get(key)
        .and_then(Value::as_i64)
        .ok_or_else(|| anyhow!("{key} is required"))
}

fn required_string(args: &Value, key: &str) -> Result<String> {
    args.get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| anyhow!("{key} is required"))
}
