use anyhow::{Result, anyhow};
use serde_json::{Map, Value, json};

use crate::api_cli::ApiTarget;
use crate::api_cli_extra::{
    PermissionsCommand, PermissionsSubcommand, ProjectCommand, ProjectCreateCommand,
    ProjectSubcommand, ProjectUpdateCommand,
};
use timely_lib::Api;
use timely_lib::api_templates_runtime;

pub async fn run_projects(api: &Api, cmd: ProjectCommand) -> Result<Value> {
    match cmd.command {
        ProjectSubcommand::List(cmd) => {
            invoke(
                api,
                "timely_list_projects",
                crate::api_commands::query_json(cmd)?,
            )
            .await
        }
        ProjectSubcommand::Get(cmd) => {
            invoke(
                api,
                "timely_get_project",
                crate::api_commands::resource_id_json(cmd, "project_id")?,
            )
            .await
        }
        ProjectSubcommand::Create(cmd) => {
            invoke(api, "timely_create_project", project_create_json(cmd)?).await
        }
        ProjectSubcommand::Update(cmd) => {
            invoke(api, "timely_update_project", project_update_json(cmd)?).await
        }
        ProjectSubcommand::Delete(cmd) => {
            invoke(
                api,
                "timely_delete_project",
                crate::api_commands::resource_id_json(cmd, "project_id")?,
            )
            .await
        }
        ProjectSubcommand::Archive(cmd) => {
            invoke(
                api,
                "timely_archive_project",
                toggle_project_json(cmd, false),
            )
            .await
        }
        ProjectSubcommand::Unarchive(cmd) => {
            invoke(
                api,
                "timely_unarchive_project",
                toggle_project_json(cmd, true),
            )
            .await
        }
    }
}

pub async fn run_permissions(api: &Api, cmd: PermissionsCommand) -> Result<Value> {
    match cmd.command {
        PermissionsSubcommand::Current(target) => {
            invoke(api, "timely_list_current_permissions", target_json(target)).await
        }
        PermissionsSubcommand::User(cmd) => {
            invoke(
                api,
                "timely_list_user_permissions",
                crate::api_commands::resource_id_json(cmd, "user_id")?,
            )
            .await
        }
    }
}

fn project_create_json(cmd: ProjectCreateCommand) -> Result<Value> {
    let mut args = target_map(cmd.target.clone());
    args.insert("name".to_string(), Value::String(cmd.name.clone()));
    args.insert(
        "rate_type".to_string(),
        Value::String(cmd.rate_type.clone()),
    );
    copy_project_fields(&mut args, &cmd)?;
    Ok(Value::Object(args))
}

fn project_update_json(cmd: ProjectUpdateCommand) -> Result<Value> {
    let mut args = target_map(cmd.target.clone());
    args.insert("project_id".to_string(), json!(cmd.id));
    insert_option(&mut args, "name", cmd.name.clone().map(Value::String));
    insert_option(
        &mut args,
        "rate_type",
        cmd.rate_type.clone().map(Value::String),
    );
    copy_project_fields(&mut args, &cmd)?;
    Ok(Value::Object(args))
}

fn toggle_project_json(cmd: crate::api_cli::ResourceGetCommand, active: bool) -> Value {
    let mut args = target_map(cmd.target);
    args.insert("project_id".to_string(), json!(cmd.id));
    args.insert("active".to_string(), Value::Bool(active));
    Value::Object(args)
}

fn copy_project_fields<T: ProjectFields>(args: &mut Map<String, Value>, cmd: &T) -> Result<()> {
    for (key, value) in [
        ("color", cmd.color().clone().map(Value::String)),
        ("description", cmd.description().clone().map(Value::String)),
        ("company_id", cmd.company_id().map(|v| json!(v))),
        ("client_id", cmd.client_id().map(|v| json!(v))),
        ("new_company", cmd.new_company().clone().map(Value::String)),
        ("hour_rate", cmd.hour_rate().map(|v| json!(v))),
        ("budget", cmd.budget().map(|v| json!(v))),
        ("budget_type", cmd.budget_type().clone().map(Value::String)),
        ("billable", cmd.billable().map(Value::Bool)),
        ("active", cmd.active().map(Value::Bool)),
        ("external_id", cmd.external_id().clone().map(Value::String)),
        (
            "budget_scope",
            cmd.budget_scope().clone().map(Value::String),
        ),
        ("send_invite", cmd.send_invite().map(Value::Bool)),
        (
            "update_hour_billable_state",
            cmd.update_hour_billable_state().map(Value::Bool),
        ),
        (
            "currency_code",
            cmd.currency_code().clone().map(Value::String),
        ),
        (
            "exchange_rate",
            cmd.exchange_rate().clone().map(Value::String),
        ),
    ] {
        insert_option(args, key, value);
    }
    insert_list(args, "team_ids", cmd.team_ids());
    insert_list(args, "label_ids", cmd.label_ids());
    insert_list(args, "required_label_ids", cmd.required_label_ids());
    if let Some(users) = parse_user_rates(cmd.user_rates())? {
        args.insert("user_rates".to_string(), users);
    }
    Ok(())
}

trait ProjectFields {
    fn color(&self) -> &Option<String>;
    fn description(&self) -> &Option<String>;
    fn company_id(&self) -> Option<i64>;
    fn client_id(&self) -> Option<i64>;
    fn new_company(&self) -> &Option<String>;
    fn hour_rate(&self) -> Option<f64>;
    fn budget(&self) -> Option<f64>;
    fn budget_type(&self) -> &Option<String>;
    fn billable(&self) -> Option<bool>;
    fn active(&self) -> Option<bool>;
    fn external_id(&self) -> &Option<String>;
    fn budget_scope(&self) -> &Option<String>;
    fn send_invite(&self) -> Option<bool>;
    fn update_hour_billable_state(&self) -> Option<bool>;
    fn currency_code(&self) -> &Option<String>;
    fn exchange_rate(&self) -> &Option<String>;
    fn team_ids(&self) -> &[i64];
    fn label_ids(&self) -> &[i64];
    fn required_label_ids(&self) -> &[i64];
    fn user_rates(&self) -> &[String];
}

macro_rules! impl_project_fields {
    () => {
        fn color(&self) -> &Option<String> {
            &self.color
        }
        fn description(&self) -> &Option<String> {
            &self.description
        }
        fn company_id(&self) -> Option<i64> {
            self.company_id
        }
        fn client_id(&self) -> Option<i64> {
            self.client_id
        }
        fn new_company(&self) -> &Option<String> {
            &self.new_company
        }
        fn hour_rate(&self) -> Option<f64> {
            self.hour_rate
        }
        fn budget(&self) -> Option<f64> {
            self.budget
        }
        fn budget_type(&self) -> &Option<String> {
            &self.budget_type
        }
        fn billable(&self) -> Option<bool> {
            self.billable
        }
        fn active(&self) -> Option<bool> {
            self.active
        }
        fn external_id(&self) -> &Option<String> {
            &self.external_id
        }
        fn budget_scope(&self) -> &Option<String> {
            &self.budget_scope
        }
        fn send_invite(&self) -> Option<bool> {
            self.send_invite
        }
        fn update_hour_billable_state(&self) -> Option<bool> {
            self.update_hour_billable_state
        }
        fn currency_code(&self) -> &Option<String> {
            &self.currency_code
        }
        fn exchange_rate(&self) -> &Option<String> {
            &self.exchange_rate
        }
        fn team_ids(&self) -> &[i64] {
            &self.team_ids
        }
        fn label_ids(&self) -> &[i64] {
            &self.label_ids
        }
        fn required_label_ids(&self) -> &[i64] {
            &self.required_label_ids
        }
        fn user_rates(&self) -> &[String] {
            &self.user_rates
        }
    };
}

impl ProjectFields for ProjectCreateCommand {
    impl_project_fields!();
}
impl ProjectFields for ProjectUpdateCommand {
    impl_project_fields!();
}

fn parse_user_rates(values: &[String]) -> Result<Option<Value>> {
    if values.is_empty() {
        return Ok(None);
    }
    let users = values
        .iter()
        .map(|value| {
            let (user_id, hour_rate) = value
                .split_once('=')
                .ok_or_else(|| anyhow!("expected --user-rate user_id=hour_rate"))?;
            Ok(json!({
                "user_id": user_id.parse::<i64>()?,
                "hour_rate": hour_rate.parse::<f64>()?,
            }))
        })
        .collect::<Result<Vec<_>, anyhow::Error>>()?;
    Ok(Some(Value::Array(users)))
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

fn insert_list(args: &mut Map<String, Value>, key: &str, values: &[i64]) {
    if !values.is_empty() {
        args.insert(key.to_string(), json!(values));
    }
}
