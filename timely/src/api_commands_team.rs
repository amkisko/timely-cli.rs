use anyhow::Result;
use serde_json::{Map, Value, json};

use crate::api_cli::ApiTarget;
use crate::api_cli_extra::{
    TeamCommand, TeamCreateCommand, TeamSearchCommand, TeamSubcommand, TeamUpdateCommand,
};
use timely_lib::Api;
use timely_lib::api_templates_runtime;

pub async fn run(api: &Api, cmd: TeamCommand) -> Result<Value> {
    match cmd.command {
        TeamSubcommand::List(cmd) => {
            invoke(
                api,
                "timely_list_teams",
                crate::api_commands::query_json(cmd)?,
            )
            .await
        }
        TeamSubcommand::Get(cmd) => {
            invoke(
                api,
                "timely_get_team",
                crate::api_commands::resource_id_json(cmd, "team_id")?,
            )
            .await
        }
        TeamSubcommand::Search(cmd) => invoke(api, "timely_search_teams", search_json(cmd)).await,
        TeamSubcommand::Create(cmd) => invoke(api, "timely_create_team", create_json(cmd)).await,
        TeamSubcommand::Update(cmd) => invoke(api, "timely_update_team", update_json(cmd)).await,
        TeamSubcommand::Delete(cmd) => {
            invoke(
                api,
                "timely_delete_team",
                crate::api_commands::resource_id_json(cmd, "team_id")?,
            )
            .await
        }
    }
}

fn search_json(cmd: TeamSearchCommand) -> Value {
    let mut args = target_map(cmd.target);
    args.insert("query".to_string(), json!({ "q": cmd.query }));
    if let Some(per_page) = cmd.per_page {
        merge_query_value(&mut args, "per_page", json!(per_page));
    }
    if let Some(page) = cmd.page {
        merge_query_value(&mut args, "page", json!(page));
    }
    Value::Object(args)
}

fn create_json(cmd: TeamCreateCommand) -> Value {
    let mut args = target_map(cmd.target);
    args.insert("name".to_string(), Value::String(cmd.name));
    copy_team_fields(
        &mut args,
        &cmd.color,
        &cmd.emoji,
        &cmd.user_ids,
        &cmd.lead_user_ids,
        &cmd.hide_hours_user_ids,
    );
    Value::Object(args)
}

fn update_json(cmd: TeamUpdateCommand) -> Value {
    let mut args = target_map(cmd.target);
    args.insert("team_id".to_string(), json!(cmd.id));
    insert_option(&mut args, "name", cmd.name.map(Value::String));
    copy_team_fields(
        &mut args,
        &cmd.color,
        &cmd.emoji,
        &cmd.user_ids,
        &cmd.lead_user_ids,
        &cmd.hide_hours_user_ids,
    );
    if let Some(add) = cmd.add_users_to_team_projects {
        merge_query_value(&mut args, "add_users_to_team_projects", Value::Bool(add));
    }
    if let Some(remove) = cmd.delete_users_from_team_projects {
        merge_query_value(
            &mut args,
            "delete_users_from_team_projects",
            Value::Bool(remove),
        );
    }
    Value::Object(args)
}

fn copy_team_fields(
    args: &mut Map<String, Value>,
    color: &Option<String>,
    emoji: &Option<String>,
    user_ids: &[i64],
    lead_user_ids: &[i64],
    hide_hours_user_ids: &[i64],
) {
    insert_option(args, "color", color.clone().map(Value::String));
    insert_option(args, "emoji", emoji.clone().map(Value::String));
    if !user_ids.is_empty() {
        args.insert("user_ids".to_string(), json!(user_ids));
    }
    if !lead_user_ids.is_empty() {
        args.insert("lead_user_ids".to_string(), json!(lead_user_ids));
    }
    if !hide_hours_user_ids.is_empty() {
        args.insert(
            "hide_hours_user_ids".to_string(),
            json!(hide_hours_user_ids),
        );
    }
}

async fn invoke(api: &Api, name: &str, args: Value) -> Result<Value> {
    api_templates_runtime::call(api, name, &args).await
}

fn target_map(target: ApiTarget) -> Map<String, Value> {
    let mut args = Map::new();
    if let Some(account_id) = target.account_id {
        args.insert("account_id".to_string(), json!(account_id));
    }
    args
}

fn insert_option(args: &mut Map<String, Value>, key: &str, value: Option<Value>) {
    if let Some(value) = value {
        args.insert(key.to_string(), value);
    }
}

fn merge_query_value(args: &mut Map<String, Value>, key: &str, value: Value) {
    let query = args.entry("query".to_string()).or_insert_with(|| json!({}));
    if let Some(query) = query.as_object_mut() {
        query.insert(key.to_string(), value);
    }
}
