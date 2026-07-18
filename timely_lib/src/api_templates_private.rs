use serde_json::{Value, json};

pub fn tools() -> Vec<Value> {
    vec![
        tool(
            "timely_experimental_memory_accounts",
            "Experimental private Memory API: fetch accounts.json.",
            json!({}),
            &[],
        ),
        tool(
            "timely_experimental_memory_identity",
            "Experimental private Memory API: fetch identity.json.",
            json!({}),
            &[],
        ),
        tool(
            "timely_experimental_memory_request",
            "Experimental private Memory API request. Undocumented and unstable.",
            json!({
                "method": {
                    "type": "string",
                    "enum": ["GET", "POST", "PUT", "PATCH", "DELETE"]
                },
                "path": { "type": "string" },
                "query": { "type": "object", "additionalProperties": { "type": "string" } },
                "body": { "type": "object" }
            }),
            &["method", "path"],
        ),
        tool(
            "timely_experimental_memory_linked_entries",
            "Best-effort extraction of Memory-linked entry IDs from Timely time entries.",
            json!({
                "account_id": { "type": "integer" },
                "query": { "type": "object", "additionalProperties": { "type": "string" } },
                "since": { "type": "string" },
                "upto": { "type": "string" },
                "day": { "type": "string" },
                "user_id": { "type": "integer" },
                "project_id": { "type": "integer" },
                "all_users": { "type": "boolean" },
                "page": { "type": "integer" },
                "per_page": { "type": "integer" }
            }),
            &[],
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
