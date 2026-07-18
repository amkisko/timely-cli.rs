use anyhow::{Result, anyhow, bail};
use serde_json::{Map, Value, json};

use crate::api::Api;
use crate::api_templates_common::{account_id, insert_query_value};
use crate::util::value_to_string_map;

pub async fn maybe_call(api: &Api, name: &str, args: &Value) -> Option<Result<Value>> {
    Some(match name {
        "timely_create_project" => create_project(api, args).await,
        "timely_update_project" => update_project(api, args).await,
        "timely_delete_project" => delete_project(api, args).await,
        "timely_archive_project" => toggle_project(api, args, false).await,
        "timely_unarchive_project" => toggle_project(api, args, true).await,
        "timely_list_current_permissions" => current_permissions(api, args).await,
        "timely_list_user_permissions" => user_permissions(api, args).await,
        "timely_reports_summary" => report_request(api, args, "/reports", None, None).await,
        "timely_reports_filter" => report_request(api, args, "/reports/filter", None, None).await,
        "timely_reports_events" => {
            report_request(api, args, "/reports/filter", None, Some("events")).await
        }
        "timely_reports_by_client" => {
            report_request(api, args, "/reports/filter", Some("clients"), None).await
        }
        "timely_reports_by_project" => {
            report_request(api, args, "/reports/filter", Some("projects"), None).await
        }
        "timely_reports_by_user" => {
            report_request(api, args, "/reports/filter", Some("users"), None).await
        }
        "timely_reports_by_team" => {
            report_request(api, args, "/reports/filter", Some("teams"), None).await
        }
        _ => return None,
    })
}

async fn create_project(api: &Api, args: &Value) -> Result<Value> {
    let account_id = account_id(api, args).await?;
    let mut project = project_body(args)?;
    project.insert("name".to_string(), json!(required_string(args, "name")?));
    project.insert(
        "rate_type".to_string(),
        json!(required_string(args, "rate_type")?),
    );
    api.send(
        "POST",
        &format!("/1.1/{account_id}/projects"),
        Vec::new(),
        Some(json!({ "project": project })),
    )
    .await
}

async fn update_project(api: &Api, args: &Value) -> Result<Value> {
    let account_id = account_id(api, args).await?;
    let project_id = required_i64(args, "project_id")?;
    let mut project = project_body(args)?;
    if let Some(name) = args.get("name") {
        project.insert("name".to_string(), name.clone());
    }
    if let Some(rate_type) = args.get("rate_type") {
        project.insert("rate_type".to_string(), rate_type.clone());
    }
    if project.is_empty() {
        bail!("timely_update_project requires at least one field to update");
    }
    api.send(
        "PUT",
        &format!("/1.1/{account_id}/projects/{project_id}"),
        Vec::new(),
        Some(json!({ "project": project })),
    )
    .await
}

async fn delete_project(api: &Api, args: &Value) -> Result<Value> {
    let account_id = account_id(api, args).await?;
    let project_id = required_i64(args, "project_id")?;
    api.send(
        "DELETE",
        &format!("/1.1/{account_id}/projects/{project_id}"),
        Vec::new(),
        None,
    )
    .await
}

async fn toggle_project(api: &Api, args: &Value, active: bool) -> Result<Value> {
    let account_id = account_id(api, args).await?;
    let project_id = required_i64(args, "project_id")?;
    let body = json!({ "project": { "active": active } });
    api.send(
        "PUT",
        &format!("/1.1/{account_id}/projects/{project_id}"),
        Vec::new(),
        Some(body),
    )
    .await
}

async fn current_permissions(api: &Api, args: &Value) -> Result<Value> {
    let account_id = account_id(api, args).await?;
    api.send(
        "GET",
        &format!("/1.1/{account_id}/users/current/permissions"),
        Vec::new(),
        None,
    )
    .await
}

async fn user_permissions(api: &Api, args: &Value) -> Result<Value> {
    let account_id = account_id(api, args).await?;
    let user_id = required_i64(args, "user_id")?;
    api.send(
        "GET",
        &format!("/1.1/{account_id}/users/{user_id}/permissions"),
        Vec::new(),
        None,
    )
    .await
}

async fn report_request(
    api: &Api,
    args: &Value,
    path: &str,
    group_by: Option<&str>,
    scope: Option<&str>,
) -> Result<Value> {
    let account_id = account_id(api, args).await?;
    let mut query = value_to_string_map(args.get("query"));
    for key in [
        "since",
        "until",
        "user_ids",
        "project_ids",
        "client_ids",
        "label_ids",
        "team_ids",
        "state_ids",
        "group_by",
        "scope",
        "billed",
    ] {
        insert_query_value(&mut query, args, key);
    }
    if let Some(group_by) = group_by {
        query.insert("group_by".to_string(), group_by.to_string());
    }
    if let Some(scope) = scope {
        query.insert("scope".to_string(), scope.to_string());
    }
    api.send(
        "GET",
        &format!("/1.1/{account_id}{path}"),
        query.into_iter().collect(),
        None,
    )
    .await
}

fn project_body(args: &Value) -> Result<Map<String, Value>> {
    let mut project = Map::new();
    for key in [
        "color",
        "description",
        "company_id",
        "client_id",
        "new_company",
        "hour_rate",
        "budget",
        "budget_type",
        "billable",
        "active",
        "external_id",
        "budget_scope",
        "send_invite",
        "update_hour_billable_state",
        "currency_code",
        "exchange_rate",
    ] {
        if let Some(value) = args.get(key) {
            project.insert(key.to_string(), value.clone());
        }
    }
    for key in ["team_ids", "label_ids", "required_label_ids"] {
        if let Some(value) = args.get(key) {
            project.insert(key.to_string(), value.clone());
        }
    }
    if let Some(users) = user_rates(args.get("user_rates"))? {
        project.insert("users".to_string(), users);
    }
    Ok(project)
}

fn user_rates(value: Option<&Value>) -> Result<Option<Value>> {
    let Some(value) = value else {
        return Ok(None);
    };
    if let Some(users) = value.as_array() {
        return Ok(Some(Value::Array(users.clone())));
    }
    let Some(map) = value.as_object() else {
        return Ok(None);
    };
    let mut users = Vec::new();
    for (user_id, hour_rate) in map {
        let user_id = user_id.parse::<i64>()?;
        let hour_rate = hour_rate
            .as_f64()
            .ok_or_else(|| anyhow!("user_rates values must be numbers"))?;
        users.push(json!({ "user_id": user_id, "hour_rate": hour_rate }));
    }
    Ok(Some(Value::Array(users)))
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
