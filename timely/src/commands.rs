use anyhow::{Result, anyhow};

use serde_json::{Value, json};

use crate::cli::{CallCommand, RequestCommand, SpecCommand, SpecSubcommand};
use timely_lib::Api;
use timely_lib::openapi::{operations, spec};
use timely_lib::util::{parse_pairs, read_body};

pub fn spec_command_value(cmd: SpecCommand) -> Result<Value> {
    let spec = spec()?;
    match cmd.command {
        SpecSubcommand::Summary => {
            let ops = operations()?;
            let schema_count = spec
                .pointer("/components/schemas")
                .and_then(serde_json::Value::as_object)
                .map(|schemas| schemas.len())
                .unwrap_or_default();
            Ok(json!({
                "title": spec["info"]["title"],
                "version": spec["info"]["version"],
                "openapi": spec["openapi"],
                "operations": ops.len(),
                "schemas": schema_count,
            }))
        }
        SpecSubcommand::Operations { tag } => {
            let items: Vec<Value> = operations()?
                .into_iter()
                .filter(|op| {
                    tag.as_ref()
                        .is_none_or(|tag| op.tags.iter().any(|item| item.eq_ignore_ascii_case(tag)))
                })
                .map(|op| {
                    json!({
                        "method": op.method,
                        "path": op.path,
                        "id": op.id,
                        "tags": op.tags,
                    })
                })
                .collect();
            Ok(Value::Array(items))
        }
        SpecSubcommand::Schemas => {
            let schemas = spec
                .pointer("/components/schemas")
                .and_then(serde_json::Value::as_object)
                .ok_or_else(|| anyhow!("OpenAPI document does not contain components.schemas"))?;
            Ok(Value::Array(
                schemas
                    .keys()
                    .map(|name| Value::String(name.clone()))
                    .collect(),
            ))
        }
    }
}

pub async fn call_command_value(api: &Api, cmd: CallCommand) -> Result<Value> {
    let op = operations()?
        .into_iter()
        .find(|op| op.id == cmd.operation)
        .ok_or_else(|| anyhow!("unknown operationId '{}'", cmd.operation))?;
    let mut params = parse_pairs(cmd.params)?;
    if let Some(account_id) = cmd.account_id {
        params.insert("account_id".to_string(), account_id.to_string());
    }
    let path = api.resolve_operation_path(&op.path, params).await?;
    api.send(
        &op.method,
        &path,
        parse_pairs(cmd.query)?.into_iter().collect(),
        read_body(cmd.body, cmd.body_file)?,
    )
    .await
}

pub async fn request_command_value(api: &Api, cmd: RequestCommand) -> Result<Value> {
    let path = api.resolve_request_path(&cmd.path).await?;
    api.send(
        &format!("{:?}", cmd.method).to_uppercase(),
        &path,
        parse_pairs(cmd.query)?.into_iter().collect(),
        read_body(cmd.body, cmd.body_file)?,
    )
    .await
}
