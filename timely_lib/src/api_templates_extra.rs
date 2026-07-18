use serde_json::{Value, json};

pub fn tools() -> Vec<Value> {
    vec![
        project_tool(
            "timely_create_project",
            "Create a Timely project.",
            &["name", "rate_type"],
        ),
        project_tool(
            "timely_update_project",
            "Update a Timely project.",
            &["project_id"],
        ),
        simple_tool(
            "timely_delete_project",
            "Delete a Timely project.",
            "project_id",
        ),
        simple_tool(
            "timely_archive_project",
            "Archive a Timely project.",
            "project_id",
        ),
        simple_tool(
            "timely_unarchive_project",
            "Unarchive a Timely project.",
            "project_id",
        ),
        tool(
            "timely_list_current_permissions",
            "List permissions for the authenticated user.",
            json!({ "account_id": { "type": "integer" } }),
            &[],
        ),
        simple_tool(
            "timely_list_user_permissions",
            "List permissions for a specific Timely user.",
            "user_id",
        ),
        report_tool("timely_reports_summary", "Get Timely report totals."),
        report_tool("timely_reports_filter", "Filter Timely reports."),
        report_tool("timely_reports_events", "Get individual report events."),
        report_tool("timely_reports_by_client", "Get reports grouped by client."),
        report_tool(
            "timely_reports_by_project",
            "Get reports grouped by project.",
        ),
        report_tool("timely_reports_by_user", "Get reports grouped by user."),
        report_tool("timely_reports_by_team", "Get reports grouped by team."),
    ]
}

fn project_tool(name: &str, description: &str, required: &[&str]) -> Value {
    tool(
        name,
        description,
        json!({
            "account_id": { "type": "integer" },
            "project_id": { "type": "integer" },
            "name": { "type": "string" },
            "rate_type": { "type": "string", "enum": ["project", "user", "non-billable"] },
            "color": { "type": "string" },
            "description": { "type": "string" },
            "company_id": { "type": "integer" },
            "client_id": { "type": "integer" },
            "new_company": { "type": "string" },
            "hour_rate": { "type": "number" },
            "budget": { "type": "number" },
            "budget_type": { "type": "string", "enum": ["H", "M"] },
            "billable": { "type": "boolean" },
            "active": { "type": "boolean" },
            "external_id": { "type": "string" },
            "budget_scope": { "type": "string", "enum": ["tag", "project"] },
            "send_invite": { "type": "boolean" },
            "update_hour_billable_state": { "type": "boolean" },
            "currency_code": { "type": "string" },
            "exchange_rate": { "type": "string" },
            "team_ids": { "type": "array", "items": { "type": "integer" } },
            "label_ids": { "type": "array", "items": { "type": "integer" } },
            "required_label_ids": { "type": "array", "items": { "type": "integer" } },
            "user_rates": { "type": "object", "additionalProperties": { "type": "number" } }
        }),
        required,
    )
}

fn simple_tool(name: &str, description: &str, key: &str) -> Value {
    tool(
        name,
        description,
        json!({
            "account_id": { "type": "integer" },
            key: { "type": "integer" }
        }),
        &[key],
    )
}

fn report_tool(name: &str, description: &str) -> Value {
    tool(
        name,
        description,
        json!({
            "account_id": { "type": "integer" },
            "query": { "type": "object", "additionalProperties": { "type": "string" } },
            "since": { "type": "string" },
            "until": { "type": "string" },
            "user_ids": { "type": "string" },
            "project_ids": { "type": "string" },
            "client_ids": { "type": "string" },
            "label_ids": { "type": "string" },
            "team_ids": { "type": "string" },
            "state_ids": { "type": "string" },
            "group_by": { "type": "string" },
            "scope": { "type": "string", "enum": ["totals", "events"] },
            "billed": { "type": "string", "enum": ["true", "false"] }
        }),
        &[],
    )
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
