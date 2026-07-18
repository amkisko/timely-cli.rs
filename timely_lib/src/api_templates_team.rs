use serde_json::{Value, json};

pub fn tools() -> Vec<Value> {
    vec![
        tool(
            "timely_list_teams",
            "List Timely teams.",
            json!({
                "account_id": { "type": "integer" },
                "query": { "type": "object", "additionalProperties": { "type": "string" } }
            }),
            &[],
        ),
        tool(
            "timely_get_team",
            "Get a Timely team by ID.",
            json!({
                "account_id": { "type": "integer" },
                "team_id": { "type": "integer" }
            }),
            &["team_id"],
        ),
        tool(
            "timely_search_teams",
            "Search Timely teams by name.",
            json!({
                "account_id": { "type": "integer" },
                "query": { "type": "object", "additionalProperties": { "type": "string" } }
            }),
            &[],
        ),
        tool(
            "timely_create_team",
            "Create a Timely team.",
            json!({
                "account_id": { "type": "integer" },
                "name": { "type": "string" },
                "color": { "type": "string" },
                "emoji": { "type": "string" },
                "user_ids": { "type": "array", "items": { "type": "integer" } },
                "lead_user_ids": { "type": "array", "items": { "type": "integer" } },
                "hide_hours_user_ids": { "type": "array", "items": { "type": "integer" } }
            }),
            &["name"],
        ),
        tool(
            "timely_update_team",
            "Update a Timely team.",
            json!({
                "account_id": { "type": "integer" },
                "team_id": { "type": "integer" },
                "name": { "type": "string" },
                "color": { "type": "string" },
                "emoji": { "type": "string" },
                "user_ids": { "type": "array", "items": { "type": "integer" } },
                "lead_user_ids": { "type": "array", "items": { "type": "integer" } },
                "hide_hours_user_ids": { "type": "array", "items": { "type": "integer" } },
                "query": { "type": "object", "additionalProperties": { "type": "string" } }
            }),
            &["team_id"],
        ),
        tool(
            "timely_delete_team",
            "Delete a Timely team.",
            json!({
                "account_id": { "type": "integer" },
                "team_id": { "type": "integer" }
            }),
            &["team_id"],
        ),
    ]
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
