use std::collections::BTreeSet;

use anyhow::{Result, anyhow, bail};
use serde_json::{Map, Value, json};

use crate::api::Api;
use crate::api_templates_common::account_id;
use crate::util::value_to_string_map;

pub async fn maybe_call(api: &Api, name: &str, args: &Value) -> Option<Result<Value>> {
    Some(match name {
        "timely_list_teams" => list_teams(api, args).await,
        "timely_get_team" => get_team(api, args).await,
        "timely_search_teams" => search_teams(api, args).await,
        "timely_create_team" => create_team(api, args).await,
        "timely_update_team" => update_team(api, args).await,
        "timely_delete_team" => delete_team(api, args).await,
        _ => return None,
    })
}

async fn list_teams(api: &Api, args: &Value) -> Result<Value> {
    list_resource(api, args, "/teams").await
}

async fn get_team(api: &Api, args: &Value) -> Result<Value> {
    get_resource(api, args, "team_id", "/teams").await
}

async fn search_teams(api: &Api, args: &Value) -> Result<Value> {
    let account_id = account_id(api, args).await?;
    let query = value_to_string_map(args.get("query"));
    api.send(
        "GET",
        &format!("/1.1/{account_id}/teams/search"),
        query.into_iter().collect(),
        None,
    )
    .await
}

async fn create_team(api: &Api, args: &Value) -> Result<Value> {
    let account_id = account_id(api, args).await?;
    let mut team = Map::new();
    team.insert("name".to_string(), json!(required_string(args, "name")?));
    copy_team_fields(&mut team, args)?;
    api.send(
        "POST",
        &format!("/1.1/{account_id}/teams"),
        Vec::new(),
        Some(json!({ "team": team })),
    )
    .await
}

async fn update_team(api: &Api, args: &Value) -> Result<Value> {
    let account_id = account_id(api, args).await?;
    let team_id = required_i64(args, "team_id")?;
    let mut team = Map::new();
    copy_team_fields(&mut team, args)?;
    if let Some(name) = args.get("name") {
        team.insert("name".to_string(), name.clone());
    }
    if team.is_empty() {
        bail!("timely_update_team requires at least one field to update");
    }
    api.send(
        "PUT",
        &format!("/1.1/{account_id}/teams/{team_id}"),
        value_to_string_map(args.get("query")).into_iter().collect(),
        Some(json!({ "team": team })),
    )
    .await
}

async fn delete_team(api: &Api, args: &Value) -> Result<Value> {
    let account_id = account_id(api, args).await?;
    let team_id = required_i64(args, "team_id")?;
    api.send(
        "DELETE",
        &format!("/1.1/{account_id}/teams/{team_id}"),
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

fn copy_team_fields(team: &mut Map<String, Value>, args: &Value) -> Result<()> {
    for key in ["color", "emoji"] {
        if let Some(value) = args.get(key) {
            team.insert(key.to_string(), value.clone());
        }
    }
    let users = team_users(args)?;
    if !users.is_empty() {
        team.insert("users".to_string(), Value::Array(users));
    }
    Ok(())
}

fn team_users(args: &Value) -> Result<Vec<Value>> {
    let users = int_list(args, "user_ids")?;
    let leads = int_list(args, "lead_user_ids")?
        .into_iter()
        .collect::<BTreeSet<_>>();
    let hidden = int_list(args, "hide_hours_user_ids")?
        .into_iter()
        .collect::<BTreeSet<_>>();
    let ids = users
        .into_iter()
        .chain(leads.iter().copied())
        .chain(hidden.iter().copied())
        .collect::<BTreeSet<_>>();
    Ok(ids
        .into_iter()
        .map(|user_id| {
            json!({
                "user_id": user_id,
                "lead": leads.contains(&user_id),
                "hide_hours": hidden.contains(&user_id),
            })
        })
        .collect())
}

fn int_list(args: &Value, key: &str) -> Result<Vec<i64>> {
    args.get(key)
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .map(|value| {
                    value
                        .as_i64()
                        .ok_or_else(|| anyhow!("{key} must contain integers"))
                })
                .collect()
        })
        .unwrap_or_else(|| Ok(Vec::new()))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn team_users_marks_leads_and_hidden_members() {
        let args = json!({
            "user_ids": [1, 2],
            "lead_user_ids": [2],
            "hide_hours_user_ids": [1]
        });
        let users = team_users(&args).unwrap();
        assert_eq!(users.len(), 2);
        assert_eq!(
            users,
            vec![
                json!({"user_id": 1, "lead": false, "hide_hours": true}),
                json!({"user_id": 2, "lead": true, "hide_hours": false}),
            ]
        );
    }
}
