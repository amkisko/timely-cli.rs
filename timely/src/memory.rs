use anyhow::{Result, anyhow};
use serde_json::{Value, json};

use crate::cli::{MemoryCommand, MemorySubcommand};
use crate::export_io;
use timely_lib::memory_db;

pub fn run_memory(cmd: MemoryCommand) -> Result<Option<Value>> {
    match cmd.command {
        MemorySubcommand::Export {
            limit,
            app,
            since,
            upto,
            include_details,
            db_path,
            file,
        } => {
            let value = json!(memory_db::export_entries(
                db_path.as_deref(),
                app.as_deref(),
                since.as_deref(),
                upto.as_deref(),
                limit,
                include_details,
            )?);
            export_io::write_json_output(&value, file.as_deref())?;
            Ok(None)
        }
        other => Ok(Some(memory_command_value(MemoryCommand {
            command: other,
        })?)),
    }
}

pub fn memory_command_value(cmd: MemoryCommand) -> Result<Value> {
    match cmd.command {
        MemorySubcommand::Status { db_path } => Ok(json!(memory_db::status(db_path.as_deref())?)),
        MemorySubcommand::Apps { limit, db_path } => {
            Ok(json!(memory_db::list_apps(db_path.as_deref(), limit)?))
        }
        MemorySubcommand::Recent {
            limit,
            app,
            include_details,
            db_path,
        } => Ok(json!(memory_db::recent_entries(
            db_path.as_deref(),
            app.as_deref(),
            limit,
            include_details,
        )?)),
        MemorySubcommand::Search {
            query,
            limit,
            app,
            include_details,
            db_path,
        } => Ok(json!(memory_db::search_entries(
            db_path.as_deref(),
            &query,
            app.as_deref(),
            limit,
            include_details,
        )?)),
        MemorySubcommand::Export {
            limit,
            app,
            since,
            upto,
            include_details,
            db_path,
            file: _,
        } => Ok(json!(memory_db::export_entries(
            db_path.as_deref(),
            app.as_deref(),
            since.as_deref(),
            upto.as_deref(),
            limit,
            include_details,
        )?)),
    }
}

pub fn tools() -> Vec<Value> {
    vec![
        tool(
            "memory_status",
            "Inspect the local Memory database and entry counts.",
            json!({
                "db_path": { "type": "string" }
            }),
            &[],
        ),
        tool(
            "memory_list_apps",
            "List apps seen in the local Memory database.",
            json!({
                "db_path": { "type": "string" },
                "limit": { "type": "integer" }
            }),
            &[],
        ),
        tool(
            "memory_recent_entries",
            "List recent local Memory entries.",
            json!({
                "db_path": { "type": "string" },
                "app": { "type": "string" },
                "limit": { "type": "integer" },
                "include_details": { "type": "boolean" }
            }),
            &[],
        ),
        tool(
            "memory_search_entries",
            "Search local Memory entries by title or details.",
            json!({
                "db_path": { "type": "string" },
                "query": { "type": "string" },
                "app": { "type": "string" },
                "limit": { "type": "integer" },
                "include_details": { "type": "boolean" }
            }),
            &["query"],
        ),
        tool(
            "memory_export_entries",
            "Export local Memory entries (higher row limit than recent/search).",
            json!({
                "db_path": { "type": "string" },
                "app": { "type": "string" },
                "since": { "type": "string" },
                "upto": { "type": "string" },
                "limit": { "type": "integer" },
                "include_details": { "type": "boolean" }
            }),
            &[],
        ),
    ]
}

pub fn call(name: &str, args: &Value) -> Result<Value> {
    match name {
        "memory_status" => Ok(json!(memory_db::status(db_path(args).as_deref())?)),
        "memory_list_apps" => Ok(json!(memory_db::list_apps(
            db_path(args).as_deref(),
            limit(args, 25),
        )?)),
        "memory_recent_entries" => Ok(json!(memory_db::recent_entries(
            db_path(args).as_deref(),
            string_arg(args, "app").as_deref(),
            limit(args, 25),
            bool_arg(args, "include_details"),
        )?)),
        "memory_search_entries" => Ok(json!(memory_db::search_entries(
            db_path(args).as_deref(),
            string_arg(args, "query")
                .as_deref()
                .ok_or_else(|| anyhow!("query is required"))?,
            string_arg(args, "app").as_deref(),
            limit(args, 25),
            bool_arg(args, "include_details"),
        )?)),
        "memory_export_entries" => Ok(json!(memory_db::export_entries(
            db_path(args).as_deref(),
            string_arg(args, "app").as_deref(),
            string_arg(args, "since").as_deref(),
            string_arg(args, "upto").as_deref(),
            export_limit(args, 1000),
            bool_arg(args, "include_details"),
        )?)),
        _ => Err(anyhow!("unknown memory command '{name}'")),
    }
}

pub fn maybe_call(name: &str, args: &Value) -> Option<Result<Value>> {
    match name {
        "memory_status"
        | "memory_list_apps"
        | "memory_recent_entries"
        | "memory_search_entries"
        | "memory_export_entries" => Some(call(name, args)),
        _ => None,
    }
}

fn tool(name: &str, description: &str, properties: Value, required: &[&str]) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": {
            "type": "object",
            "properties": properties,
            "required": required,
        }
    })
}

fn db_path(args: &Value) -> Option<String> {
    string_arg(args, "db_path")
}

fn string_arg(args: &Value, key: &str) -> Option<String> {
    args.get(key).and_then(Value::as_str).map(str::to_string)
}

fn bool_arg(args: &Value, key: &str) -> bool {
    args.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn limit(args: &Value, default: usize) -> usize {
    args.get("limit")
        .and_then(Value::as_u64)
        .map(|limit| memory_db::normalized_limit(limit as usize))
        .unwrap_or(default)
}

fn export_limit(args: &Value, default: usize) -> usize {
    args.get("limit")
        .and_then(Value::as_u64)
        .map(|limit| memory_db::normalized_export_limit(limit as usize))
        .unwrap_or(default)
}
