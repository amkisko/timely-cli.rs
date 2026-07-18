use anyhow::Result;
use serde_json::{Value, json};

use crate::api::Api;
use crate::api_templates_extra;
use crate::api_templates_private;
use crate::api_templates_runtime;
use crate::api_templates_team;

pub fn tools() -> Vec<Value> {
    let mut tools = vec![
        tool(
            "timely_me",
            "Get the authenticated Timely user.",
            json!({
                "account_id": { "type": "integer" }
            }),
            &[],
        ),
        tool(
            "timely_list_clients",
            "List Timely clients.",
            json!({
                "account_id": { "type": "integer" },
                "query": { "type": "object", "additionalProperties": { "type": "string" } }
            }),
            &[],
        ),
        tool(
            "timely_get_client",
            "Get a Timely client by ID.",
            json!({
                "account_id": { "type": "integer" },
                "client_id": { "type": "integer" }
            }),
            &["client_id"],
        ),
        tool(
            "timely_list_projects",
            "List Timely projects.",
            json!({
                "account_id": { "type": "integer" },
                "query": { "type": "object", "additionalProperties": { "type": "string" } }
            }),
            &[],
        ),
        tool(
            "timely_get_project",
            "Get a Timely project by ID.",
            json!({
                "account_id": { "type": "integer" },
                "project_id": { "type": "integer" }
            }),
            &["project_id"],
        ),
        tool(
            "timely_list_users",
            "List Timely users.",
            json!({
                "account_id": { "type": "integer" },
                "query": { "type": "object", "additionalProperties": { "type": "string" } }
            }),
            &[],
        ),
        tool(
            "timely_get_user",
            "Get a Timely user by ID.",
            json!({
                "account_id": { "type": "integer" },
                "user_id": { "type": "integer" }
            }),
            &["user_id"],
        ),
        tool(
            "timely_list_time_entries",
            "List time entries. Defaults to the authenticated user's entries when user_id is omitted.",
            json!({
                "account_id": { "type": "integer" },
                "query": { "type": "object", "additionalProperties": { "type": "string" } },
                "since": { "type": "string" },
                "upto": { "type": "string" },
                "day": { "type": "string" },
                "user_id": { "type": "integer" },
                "project_id": { "type": "integer" },
                "per_page": { "type": "integer" },
                "page": { "type": "integer" },
                "sort": { "type": "string", "enum": ["day", "created_at", "updated_at"] },
                "order": { "type": "string", "enum": ["asc", "desc"] },
                "all_users": { "type": "boolean" },
                "include_linked_metadata": { "type": "boolean" }
            }),
            &[],
        ),
        tool(
            "timely_get_time_entry",
            "Get a time entry by ID.",
            json!({
                "account_id": { "type": "integer" },
                "time_entry_id": { "type": "integer" }
            }),
            &["time_entry_id"],
        ),
        tool(
            "timely_create_time_entry",
            "Create a time entry.",
            json!({
                "account_id": { "type": "integer" },
                "project_id": { "type": "integer" },
                "day": { "type": "string" },
                "hours": { "type": "number" },
                "minutes": { "type": "integer" },
                "seconds": { "type": "integer" },
                "note": { "type": "string" },
                "from": { "type": "string" },
                "to": { "type": "string" },
                "label_ids": { "type": "array", "items": { "type": "integer" } }
            }),
            &["project_id", "day"],
        ),
        tool(
            "timely_update_time_entry",
            "Update a time entry.",
            json!({
                "account_id": { "type": "integer" },
                "time_entry_id": { "type": "integer" },
                "project_id": { "type": "integer" },
                "day": { "type": "string" },
                "hours": { "type": "number" },
                "minutes": { "type": "integer" },
                "seconds": { "type": "integer" },
                "note": { "type": "string" },
                "from": { "type": "string" },
                "to": { "type": "string" },
                "label_ids": { "type": "array", "items": { "type": "integer" } }
            }),
            &["time_entry_id"],
        ),
        tool(
            "timely_delete_time_entry",
            "Delete a time entry.",
            json!({
                "account_id": { "type": "integer" },
                "time_entry_id": { "type": "integer" }
            }),
            &["time_entry_id"],
        ),
        tool(
            "timely_start_timer",
            "Start a running timer on an existing time entry.",
            json!({
                "account_id": { "type": "integer" },
                "time_entry_id": { "type": "integer" }
            }),
            &["time_entry_id"],
        ),
        tool(
            "timely_stop_timer",
            "Stop a running timer on an existing time entry.",
            json!({
                "account_id": { "type": "integer" },
                "time_entry_id": { "type": "integer" }
            }),
            &["time_entry_id"],
        ),
        tool(
            "timely_list_labels",
            "List Timely labels.",
            json!({
                "account_id": { "type": "integer" },
                "query": { "type": "object", "additionalProperties": { "type": "string" } }
            }),
            &[],
        ),
        tool(
            "timely_get_label",
            "Get a Timely label by ID.",
            json!({
                "account_id": { "type": "integer" },
                "label_id": { "type": "integer" }
            }),
            &["label_id"],
        ),
        tool(
            "timely_list_tasks",
            "List Timely tasks.",
            json!({
                "account_id": { "type": "integer" },
                "query": { "type": "object", "additionalProperties": { "type": "string" } }
            }),
            &[],
        ),
        tool(
            "timely_get_task",
            "Get a Timely task by ID.",
            json!({
                "account_id": { "type": "integer" },
                "task_id": { "type": "integer" }
            }),
            &["task_id"],
        ),
    ];
    tools.extend(api_templates_team::tools());
    tools.extend(api_templates_extra::tools());
    tools.extend(api_templates_private::tools());
    tools
}

pub async fn maybe_call(api: &Api, name: &str, args: &Value) -> Option<Result<Value>> {
    api_templates_runtime::maybe_call(api, name, args).await
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
