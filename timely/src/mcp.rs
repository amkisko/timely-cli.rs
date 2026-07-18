use std::io::{BufRead, Write};

use anyhow::{Result, anyhow};
use serde_json::{Value, json};

use crate::memory;
use timely_lib::Api;
use timely_lib::api_templates;
use timely_lib::openapi::operations;
use timely_lib::util::{object_to_pairs, value_to_string_map};

pub async fn run(api: Api) -> Result<()> {
    let mut stdin = std::io::stdin().lock();
    while let Some(message) = read_message(&mut stdin)? {
        let response = handle_message(&api, message).await;
        write_message(&response)?;
    }
    Ok(())
}

async fn handle_message(api: &Api, message: Value) -> Value {
    let id = message.get("id").cloned().unwrap_or(Value::Null);
    let method = message.get("method").and_then(Value::as_str).unwrap_or("");
    let result = match method {
        "initialize" => Ok(initialize_result()),
        "notifications/initialized" => return Value::Null,
        "tools/list" => tools(),
        "tools/call" => tool_call(api, message.get("params").cloned().unwrap_or_default()).await,
        _ => Err(anyhow!("unsupported MCP method '{method}'")),
    };
    match result {
        Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        Err(err) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": -32000, "message": err.to_string() }
        }),
    }
}

fn initialize_result() -> Value {
    json!({
        "protocolVersion": "2024-11-05",
        "capabilities": { "tools": {} },
        "serverInfo": { "name": "timely-cli", "version": env!("CARGO_PKG_VERSION") }
    })
}

fn tools() -> Result<Value> {
    let mut tools = vec![json!({
        "name": "timely_request",
        "description": concat!(
            "Make a raw authenticated Timely API request. ",
            "Use {account_id} in the path to auto-fill your default workspace."
        ),
        "inputSchema": {
            "type": "object",
            "properties": {
                "method": { "type": "string", "enum": ["GET", "POST", "PUT", "PATCH", "DELETE"] },
                "path": { "type": "string" },
                "query": { "type": "object", "additionalProperties": { "type": "string" } },
                "body": { "type": "object" }
            },
            "required": ["method", "path"]
        }
    })];
    tools.extend(api_templates::tools());
    tools.extend(memory::tools());
    for op in operations()? {
        tools.push(json!({
            "name": format!("timely_openapi_{}", op.id),
            "description": if op.summary.is_empty() {
                format!("Generated from OpenAPI: {} {}", op.method, op.path)
            } else {
                format!("Generated from OpenAPI: {}", op.summary)
            },
            "inputSchema": {
                "type": "object",
                "properties": {
                    "account_id": { "type": "integer" },
                    "params": { "type": "object", "additionalProperties": { "type": "string" } },
                    "query": { "type": "object", "additionalProperties": { "type": "string" } },
                    "body": { "type": "object" }
                }
            }
        }));
    }
    Ok(json!({ "tools": tools }))
}

async fn tool_call(api: &Api, params: Value) -> Result<Value> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("tools/call missing name"))?;
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let response = if let Some(result) = memory::maybe_call(name, &arguments) {
        result?
    } else if let Some(result) = api_templates::maybe_call(api, name, &arguments).await {
        result?
    } else if name == "timely_request" {
        raw_request(api, &arguments).await?
    } else {
        operation_request(api, name, &arguments).await?
    };
    Ok(json!({
        "content": [{ "type": "text", "text": serde_json::to_string_pretty(&response)? }]
    }))
}

async fn raw_request(api: &Api, arguments: &Value) -> Result<Value> {
    let method = arguments
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or("GET");
    let path = arguments
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("timely_request requires path"))?;
    let path = api.resolve_request_path(path).await?;
    api.send(
        method,
        &path,
        object_to_pairs(arguments.get("query")),
        arguments.get("body").cloned(),
    )
    .await
}

async fn operation_request(api: &Api, name: &str, arguments: &Value) -> Result<Value> {
    let op_id = name
        .strip_prefix("timely_openapi_")
        .or_else(|| name.strip_prefix("timely_"))
        .ok_or_else(|| anyhow!("unknown tool '{name}'"))?;
    let op = operations()?
        .into_iter()
        .find(|op| op.id == op_id)
        .ok_or_else(|| anyhow!("unknown operation tool '{name}'"))?;
    let mut path_params = value_to_string_map(arguments.get("params"));
    if let Some(account_id) = arguments.get("account_id").and_then(Value::as_i64) {
        path_params.insert("account_id".to_string(), account_id.to_string());
    }
    let path = api.resolve_operation_path(&op.path, path_params).await?;
    api.send(
        &op.method,
        &path,
        object_to_pairs(arguments.get("query")),
        arguments.get("body").cloned(),
    )
    .await
}

pub fn read_message(reader: &mut impl BufRead) -> Result<Option<Value>> {
    let Some(length) = read_content_length(reader)? else {
        return Ok(None);
    };
    let mut body = vec![0_u8; length];
    reader.read_exact(&mut body)?;
    Ok(Some(serde_json::from_slice(&body)?))
}

fn read_content_length(reader: &mut impl BufRead) -> Result<Option<usize>> {
    let mut content_length = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            return Ok(content_length);
        }
        if let Some(value) = trimmed.strip_prefix("Content-Length:") {
            content_length = Some(value.trim().parse::<usize>()?);
        }
    }
}

pub fn write_message(value: &Value) -> Result<Vec<u8>> {
    if value.is_null() {
        return Ok(Vec::new());
    }
    let body = serde_json::to_vec(value)?;
    let mut out = Vec::new();
    write!(out, "Content-Length: {}\r\n\r\n", body.len())?;
    out.write_all(&body)?;
    std::io::stdout().write_all(&out)?;
    std::io::stdout().flush()?;
    Ok(out)
}
