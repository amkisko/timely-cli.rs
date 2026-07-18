use anyhow::Result;
use serde_json::{Map, Value, json};

use crate::api_cli::ApiTarget;
use crate::api_cli_extra::{ReportQueryCommand, ReportsCommand, ReportsSubcommand};
use timely_lib::Api;
use timely_lib::api_templates_runtime;
use timely_lib::util::parse_pairs;

pub async fn run(api: &Api, cmd: ReportsCommand) -> Result<Value> {
    match cmd.command {
        ReportsSubcommand::Summary(cmd) => {
            invoke(api, "timely_reports_summary", report_json(cmd, None, None)?).await
        }
        ReportsSubcommand::Filter(cmd) => {
            invoke(api, "timely_reports_filter", report_json(cmd, None, None)?).await
        }
        ReportsSubcommand::Events(cmd) => {
            invoke(
                api,
                "timely_reports_events",
                report_json(cmd, None, Some("events"))?,
            )
            .await
        }
        ReportsSubcommand::ByClient(cmd) => {
            invoke(
                api,
                "timely_reports_by_client",
                report_json(cmd, Some("clients"), None)?,
            )
            .await
        }
        ReportsSubcommand::ByProject(cmd) => {
            invoke(
                api,
                "timely_reports_by_project",
                report_json(cmd, Some("projects"), None)?,
            )
            .await
        }
        ReportsSubcommand::ByUser(cmd) => {
            invoke(
                api,
                "timely_reports_by_user",
                report_json(cmd, Some("users"), None)?,
            )
            .await
        }
        ReportsSubcommand::ByTeam(cmd) => {
            invoke(
                api,
                "timely_reports_by_team",
                report_json(cmd, Some("teams"), None)?,
            )
            .await
        }
    }
}

fn report_json(
    cmd: ReportQueryCommand,
    group_by: Option<&str>,
    scope: Option<&str>,
) -> Result<Value> {
    let mut args = target_map(cmd.target);
    let query = parse_pairs(cmd.query)?;
    if !query.is_empty() {
        args.insert("query".to_string(), json!(query));
    }
    for (key, value) in [
        ("since", cmd.since.map(Value::String)),
        ("until", cmd.until.map(Value::String)),
        ("user_ids", cmd.user_ids.map(Value::String)),
        ("project_ids", cmd.project_ids.map(Value::String)),
        ("client_ids", cmd.client_ids.map(Value::String)),
        ("label_ids", cmd.label_ids.map(Value::String)),
        ("team_ids", cmd.team_ids.map(Value::String)),
        ("state_ids", cmd.state_ids.map(Value::String)),
        ("group_by", cmd.group_by.map(Value::String)),
        ("scope", cmd.scope.map(Value::String)),
        ("billed", cmd.billed.map(Value::String)),
    ] {
        insert_option(&mut args, key, value);
    }
    if let Some(group_by) = group_by {
        args.insert("group_by".to_string(), Value::String(group_by.to_string()));
    }
    if let Some(scope) = scope {
        args.insert("scope".to_string(), Value::String(scope.to_string()));
    }
    Ok(Value::Object(args))
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
