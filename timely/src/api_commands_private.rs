use anyhow::Result;
use serde_json::{Map, Value, json};

use crate::api_cli::ApiTarget;
use crate::api_cli_private::{
    ExperimentalCommand, ExperimentalLinkedEntriesCommand, ExperimentalMemorySubcommand,
    ExperimentalRequestCommand, ExperimentalSubcommand,
};
use timely_lib::Api;
use timely_lib::api_templates_runtime;
use timely_lib::util::{parse_pairs, read_body};

pub async fn run(api: &Api, cmd: ExperimentalCommand) -> Result<Value> {
    match cmd.command {
        ExperimentalSubcommand::Memory(memory) => match memory.command {
            ExperimentalMemorySubcommand::Accounts(target) => {
                invoke(
                    api,
                    "timely_experimental_memory_accounts",
                    target_json(target),
                )
                .await
            }
            ExperimentalMemorySubcommand::Identity(target) => {
                invoke(
                    api,
                    "timely_experimental_memory_identity",
                    target_json(target),
                )
                .await
            }
            ExperimentalMemorySubcommand::LinkedEntries(cmd) => {
                invoke(
                    api,
                    "timely_experimental_memory_linked_entries",
                    linked_json(cmd)?,
                )
                .await
            }
            ExperimentalMemorySubcommand::Request(cmd) => request(api, cmd).await,
        },
    }
}

async fn request(api: &Api, cmd: ExperimentalRequestCommand) -> Result<Value> {
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

fn linked_json(cmd: ExperimentalLinkedEntriesCommand) -> Result<Value> {
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
    insert_option(&mut args, "page", cmd.page.map(|v| json!(v)));
    insert_option(&mut args, "per_page", cmd.per_page.map(|v| json!(v)));
    if cmd.all_users {
        args.insert("all_users".to_string(), Value::Bool(true));
    }
    Ok(Value::Object(args))
}

async fn invoke(api: &Api, name: &str, args: Value) -> Result<Value> {
    api_templates_runtime::call(api, name, &args).await
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
