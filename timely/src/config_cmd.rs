//! `timely config` subcommand handlers.

use serde_json::{Value, json};

use timely_lib::{
    ConfigSource, config_file_path, friendly_config_key, get_config_entry, list_config_entries,
    set_config_entry, timely_home, unset_config_entry,
};

use crate::cli_config::{ConfigCommand, ConfigSubcommand};
use crate::cli_util;
use crate::output::OutputMode;
use crate::run_context::RunContext;

pub fn run_config(command: ConfigCommand, context: &RunContext) -> Result<Option<Value>, String> {
    match command.command {
        ConfigSubcommand::Path => Ok(Some(path_config_value()?)),
        ConfigSubcommand::List => Ok(Some(list_config_value()?)),
        ConfigSubcommand::Get { key } => {
            let entry = get_config_entry(&key)?;
            if entry.value.is_none() {
                return Err(format!("config key not set: {key}"));
            }
            Ok(Some(
                serde_json::to_value(entry).map_err(|error| error.to_string())?,
            ))
        }
        ConfigSubcommand::Set { key, value } => {
            if context.confirm.dry_run {
                let resolved = friendly_config_key(&key)?;
                if !context.error_context.quiet {
                    cli_util::user_notice(false, &format!("would set {resolved}={value}"));
                }
                return Ok(structured_or_none(
                    context,
                    json!({
                        "key": resolved,
                        "value": value,
                        "dry_run": true
                    }),
                ));
            }
            let entry = set_config_entry(&key, &value)?;
            if !context.error_context.quiet {
                cli_util::user_notice(false, &format!("set {}", entry.key));
            }
            Ok(structured_or_none(
                context,
                serde_json::to_value(entry).map_err(|error| error.to_string())?,
            ))
        }
        ConfigSubcommand::Unset { key } => {
            if context.confirm.dry_run {
                if !context.error_context.quiet {
                    cli_util::user_notice(false, &format!("would unset {key}"));
                }
                return Ok(structured_or_none(
                    context,
                    json!({ "key": key, "dry_run": true }),
                ));
            }
            unset_config_entry(&key)?;
            if !context.error_context.quiet {
                cli_util::user_notice(false, &format!("unset {key}"));
            }
            Ok(structured_or_none(
                context,
                json!({ "key": key, "unset": true }),
            ))
        }
    }
}

fn structured_or_none(context: &RunContext, value: Value) -> Option<Value> {
    match context.emit.mode {
        OutputMode::JsonCompact | OutputMode::JsonPretty => Some(value),
        OutputMode::HumanPlain | OutputMode::ScriptPlain => None,
    }
}

pub fn config_command_value(command: ConfigCommand) -> Result<Value, String> {
    match command.command {
        ConfigSubcommand::List => list_config_value(),
        ConfigSubcommand::Get { key } => {
            let entry = get_config_entry(&key)?;
            if entry.value.is_none() {
                return Err(format!("config key not set: {key}"));
            }
            serde_json::to_value(entry).map_err(|error| error.to_string())
        }
        ConfigSubcommand::Path => path_config_value(),
        ConfigSubcommand::Set { .. } | ConfigSubcommand::Unset { .. } => {
            Err("config set/unset cannot run inside batch (state-changing)".to_string())
        }
    }
}

pub fn format_plain_config(value: &Value) -> Option<String> {
    if let Some(home) = value.get("timely_home").and_then(Value::as_str) {
        let config_path = value.get("config_path").and_then(Value::as_str)?;
        return Some(format!("{home}\n{config_path}"));
    }
    if let Some(entries) = value.get("entries").and_then(Value::as_array) {
        let mut lines = Vec::new();
        for entry in entries {
            let key = entry.get("key").and_then(Value::as_str).unwrap_or("");
            let value = entry.get("value").and_then(Value::as_str);
            let source = entry
                .get("source")
                .and_then(Value::as_str)
                .and_then(source_label_from_str);
            match (value, source) {
                (Some(value), Some(source)) => lines.push(format!("{key}={value} ({source})")),
                (Some(value), None) => lines.push(format!("{key}={value}")),
                (None, _) => lines.push(format!("{key}=")),
            }
        }
        return Some(lines.join("\n"));
    }
    if let (Some(key), Some(entry_value)) = (
        value.get("key").and_then(Value::as_str),
        value.get("value").and_then(Value::as_str),
    ) {
        if value.get("dry_run").and_then(Value::as_bool) == Some(true) {
            return Some(format!("would set {key}={entry_value}"));
        }
        let source = value
            .get("source")
            .and_then(Value::as_str)
            .and_then(source_label_from_str);
        return Some(match source {
            Some(source) => format!("{entry_value} ({source})"),
            None => entry_value.to_string(),
        });
    }
    None
}

fn path_config_value() -> Result<Value, String> {
    let home =
        timely_home().ok_or_else(|| "could not resolve timely home directory".to_string())?;
    let config_path = config_file_path()?;
    Ok(json!({
        "timely_home": home.display().to_string(),
        "config_path": config_path.display().to_string(),
    }))
}

fn list_config_value() -> Result<Value, String> {
    let entries = list_config_entries()?;
    Ok(json!({ "entries": entries }))
}

fn source_label_from_str(source: &str) -> Option<&'static str> {
    match source {
        "env" => Some(source_label(ConfigSource::Env)),
        "local_file" => Some(source_label(ConfigSource::LocalFile)),
        "file" => Some(source_label(ConfigSource::File)),
        "project_file" => Some(source_label(ConfigSource::ProjectFile)),
        _ => None,
    }
}

fn source_label(source: ConfigSource) -> &'static str {
    match source {
        ConfigSource::Env => "env",
        ConfigSource::LocalFile => "local",
        ConfigSource::File => "file",
        ConfigSource::ProjectFile => "project",
    }
}
