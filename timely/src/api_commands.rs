use anyhow::Result;
use serde_json::{Map, Value, json};

use crate::api_cli::{
    ApiCommand, ApiRawCommand, ApiSubcommand, ApiTarget, QueryListCommand, ResourceCommand,
    ResourceGetCommand, ResourceSubcommand, TimeEntriesCommand, TimeEntriesSubcommand,
    TimeEntryCreateCommand, TimeEntryListCommand, TimeEntryUpdateCommand,
};
use crate::api_cli_extra::{ProjectCommand, ProjectSubcommand, TeamCommand, TeamSubcommand};
use crate::api_commands_extra;
use crate::api_commands_private;
use crate::api_commands_reports;
use crate::api_commands_team;
use crate::confirm::{self, ConfirmOptions, GuardResult};
use timely_lib::Api;
use timely_lib::api_templates_runtime;
use timely_lib::util::{parse_pairs, read_body};

pub async fn api_command_value(
    api: &Api,
    cmd: ApiCommand,
    confirm_options: &ConfirmOptions,
) -> Result<Value> {
    if let Some(action) = destructive_action(&cmd.command) {
        match confirm::guard(&action, confirm_options)? {
            GuardResult::Proceed => {}
            GuardResult::DryRun => {
                return Ok(json!({ "dry_run": true, "action": action }));
            }
        }
    }
    match cmd.command {
        ApiSubcommand::Me(target) => invoke(api, "timely_me", target_json(target)).await,
        ApiSubcommand::Clients(cmd) => run_resource(api, cmd, "client").await,
        ApiSubcommand::Teams(cmd) => api_commands_team::run(api, cmd).await,
        ApiSubcommand::Projects(cmd) => api_commands_extra::run_projects(api, cmd).await,
        ApiSubcommand::Users(cmd) => run_resource(api, cmd, "user").await,
        ApiSubcommand::Labels(cmd) => run_resource(api, cmd, "label").await,
        ApiSubcommand::Tasks(cmd) => run_resource(api, cmd, "task").await,
        ApiSubcommand::Permissions(cmd) => api_commands_extra::run_permissions(api, cmd).await,
        ApiSubcommand::Experimental(cmd) => api_commands_private::run(api, cmd).await,
        ApiSubcommand::Reports(cmd) => api_commands_reports::run(api, cmd).await,
        ApiSubcommand::TimeEntries(cmd) => run_time_entries(api, cmd).await,
        ApiSubcommand::Raw(cmd) => run_raw(api, cmd).await,
    }
}

async fn run_resource(api: &Api, cmd: ResourceCommand, kind: &str) -> Result<Value> {
    match cmd.command {
        ResourceSubcommand::List(cmd) => {
            invoke(api, &format!("timely_list_{kind}s"), query_json(cmd)?).await
        }
        ResourceSubcommand::Get(cmd) => {
            invoke(
                api,
                &format!("timely_get_{kind}"),
                resource_id_json(cmd, &format!("{kind}_id"))?,
            )
            .await
        }
    }
}

async fn run_time_entries(api: &Api, cmd: TimeEntriesCommand) -> Result<Value> {
    match cmd.command {
        TimeEntriesSubcommand::List(cmd) => {
            invoke(api, "timely_list_time_entries", time_entry_list_json(cmd)?).await
        }
        TimeEntriesSubcommand::Get(cmd) => {
            invoke(api, "timely_get_time_entry", time_entry_id_json(cmd)?).await
        }
        TimeEntriesSubcommand::Create(cmd) => {
            invoke(api, "timely_create_time_entry", time_entry_create_json(cmd)).await
        }
        TimeEntriesSubcommand::Update(cmd) => {
            invoke(api, "timely_update_time_entry", time_entry_update_json(cmd)).await
        }
        TimeEntriesSubcommand::Delete(cmd) => {
            invoke(api, "timely_delete_time_entry", time_entry_id_json(cmd)?).await
        }
        TimeEntriesSubcommand::Start(cmd) => {
            invoke(api, "timely_start_timer", time_entry_id_json(cmd)?).await
        }
        TimeEntriesSubcommand::Stop(cmd) => {
            invoke(api, "timely_stop_timer", time_entry_id_json(cmd)?).await
        }
    }
}

async fn run_raw(api: &Api, cmd: ApiRawCommand) -> Result<Value> {
    let path = if let Some(account_id) = cmd.target.account_id {
        cmd.path.replace("{account_id}", &account_id.to_string())
    } else {
        api.resolve_request_path(&cmd.path).await?
    };
    api.send(
        &format!("{:?}", cmd.method).to_uppercase(),
        &path,
        parse_pairs(cmd.query)?.into_iter().collect(),
        read_body(cmd.body, cmd.body_file)?,
    )
    .await
}

async fn invoke(api: &Api, name: &str, args: Value) -> Result<Value> {
    api_templates_runtime::call(api, name, &args).await
}

pub(crate) fn query_json(cmd: QueryListCommand) -> Result<Value> {
    let mut args = target_map(cmd.target);
    let query = parse_pairs(cmd.query)?;
    if !query.is_empty() {
        args.insert("query".to_string(), json!(query));
    }
    Ok(Value::Object(args))
}

pub(crate) fn resource_id_json(cmd: ResourceGetCommand, key: &str) -> Result<Value> {
    let mut args = target_map(cmd.target);
    args.insert(key.to_string(), json!(cmd.id));
    Ok(Value::Object(args))
}

fn time_entry_id_json(cmd: ResourceGetCommand) -> Result<Value> {
    let mut args = target_map(cmd.target);
    args.insert("time_entry_id".to_string(), json!(cmd.id));
    Ok(Value::Object(args))
}

fn time_entry_list_json(cmd: TimeEntryListCommand) -> Result<Value> {
    let mut args = target_map(cmd.target);
    let query = parse_pairs(cmd.query)?;
    if !query.is_empty() {
        args.insert("query".to_string(), json!(query));
    }
    insert_option(&mut args, "since", cmd.since.map(Value::String));
    insert_option(&mut args, "upto", cmd.upto.map(Value::String));
    insert_option(&mut args, "day", cmd.day.map(Value::String));
    insert_option(&mut args, "user_id", cmd.user_id.map(|v| json!(v)));
    insert_option(&mut args, "project_id", cmd.project_id.map(|v| json!(v)));
    insert_option(&mut args, "per_page", cmd.per_page.map(|v| json!(v)));
    insert_option(&mut args, "page", cmd.page.map(|v| json!(v)));
    insert_option(&mut args, "sort", cmd.sort.map(Value::String));
    insert_option(&mut args, "order", cmd.order.map(Value::String));
    if cmd.all_users {
        args.insert("all_users".to_string(), Value::Bool(true));
    }
    if cmd.include_linked_metadata {
        args.insert("include_linked_metadata".to_string(), Value::Bool(true));
    }
    Ok(Value::Object(args))
}

fn time_entry_create_json(cmd: TimeEntryCreateCommand) -> Value {
    let mut args = target_map(cmd.target);
    args.insert("project_id".to_string(), json!(cmd.project_id));
    args.insert("day".to_string(), Value::String(cmd.day));
    insert_option(&mut args, "hours", cmd.hours.map(|v| json!(v)));
    insert_option(&mut args, "minutes", cmd.minutes.map(|v| json!(v)));
    insert_option(&mut args, "seconds", cmd.seconds.map(|v| json!(v)));
    insert_option(&mut args, "note", cmd.note.map(Value::String));
    insert_option(&mut args, "from", cmd.from.map(Value::String));
    insert_option(&mut args, "to", cmd.to.map(Value::String));
    if !cmd.label_ids.is_empty() {
        args.insert("label_ids".to_string(), json!(cmd.label_ids));
    }
    Value::Object(args)
}

fn time_entry_update_json(cmd: TimeEntryUpdateCommand) -> Value {
    let mut args = target_map(cmd.target);
    args.insert("time_entry_id".to_string(), json!(cmd.id));
    insert_option(&mut args, "project_id", cmd.project_id.map(|v| json!(v)));
    insert_option(&mut args, "day", cmd.day.map(Value::String));
    insert_option(&mut args, "hours", cmd.hours.map(|v| json!(v)));
    insert_option(&mut args, "minutes", cmd.minutes.map(|v| json!(v)));
    insert_option(&mut args, "seconds", cmd.seconds.map(|v| json!(v)));
    insert_option(&mut args, "note", cmd.note.map(Value::String));
    insert_option(&mut args, "from", cmd.from.map(Value::String));
    insert_option(&mut args, "to", cmd.to.map(Value::String));
    if !cmd.label_ids.is_empty() {
        args.insert("label_ids".to_string(), json!(cmd.label_ids));
    }
    Value::Object(args)
}

fn target_json(target: ApiTarget) -> Value {
    Value::Object(target_map(target))
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

fn destructive_action(command: &ApiSubcommand) -> Option<String> {
    match command {
        ApiSubcommand::TimeEntries(TimeEntriesCommand {
            command: TimeEntriesSubcommand::Delete(cmd),
        }) => Some(format!("delete time entry {}", cmd.id)),
        ApiSubcommand::Teams(TeamCommand {
            command: TeamSubcommand::Delete(cmd),
        }) => Some(format!("delete team {}", cmd.id)),
        ApiSubcommand::Projects(ProjectCommand {
            command: ProjectSubcommand::Delete(cmd),
        }) => Some(format!("delete project {}", cmd.id)),
        ApiSubcommand::Projects(ProjectCommand {
            command: ProjectSubcommand::Archive(cmd),
        }) => Some(format!("archive project {}", cmd.id)),
        _ => None,
    }
}
