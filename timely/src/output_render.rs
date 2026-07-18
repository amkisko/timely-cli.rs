//! Human-readable and script-stable output rendering.

use serde_json::Value;
use std::fmt::Write;

pub fn format_human_plain(value: &Value, max_width: usize, use_color: bool) -> String {
    let mut output = String::new();
    format_human_plain_impl(value, &mut output, 0, max_width, use_color);
    output
}

pub fn format_script_plain(value: &Value) -> String {
    match value {
        Value::Array(items) if items.iter().all(Value::is_object) => items
            .iter()
            .map(format_script_record)
            .collect::<Vec<_>>()
            .join("\n"),
        Value::Object(_) => format_script_record(value),
        other => other.to_string(),
    }
}

fn format_human_plain_impl(
    value: &Value,
    output: &mut String,
    indent: usize,
    max_width: usize,
    use_color: bool,
) {
    let padding = "  ".repeat(indent);
    match value {
        Value::Null => {
            let _ = writeln!(output, "{padding}null");
        }
        Value::Bool(boolean) => {
            let _ = writeln!(output, "{padding}{boolean}");
        }
        Value::Number(number) => {
            let _ = writeln!(output, "{padding}{number}");
        }
        Value::String(text) => {
            let _ = writeln!(output, "{padding}{text}");
        }
        Value::Array(items) => {
            if items.is_empty() {
                let _ = writeln!(output, "{padding}<empty>");
                return;
            }
            if items.iter().all(Value::is_object) {
                let keys = union_object_keys(items);
                if !keys.is_empty() {
                    render_table(output, &padding, &keys, items, max_width, use_color);
                    return;
                }
            }
            for (index, item) in items.iter().enumerate() {
                if item.is_object() || item.is_array() {
                    let _ = writeln!(output, "{padding}[{}]", index + 1);
                    format_human_plain_impl(item, output, indent + 1, max_width, use_color);
                } else {
                    let _ = writeln!(output, "{padding}{item}");
                }
            }
        }
        Value::Object(map) => {
            for (key, nested) in map {
                if nested.is_object() || nested.is_array() {
                    let _ = writeln!(output, "{padding}{key}:");
                    format_human_plain_impl(nested, output, indent + 1, max_width, use_color);
                } else {
                    let rendered = as_short_str(nested).unwrap_or_else(|| "null".to_string());
                    let _ = writeln!(output, "{padding}{key}: {rendered}");
                }
            }
        }
    }
}

fn render_table(
    output: &mut String,
    padding: &str,
    keys: &[String],
    rows: &[Value],
    max_width: usize,
    use_color: bool,
) {
    let column_width = ((max_width.saturating_sub(keys.len())) / keys.len().max(1)).clamp(8, 24);
    let header = keys
        .iter()
        .map(|key| format!("{key:>column_width$}"))
        .collect::<Vec<_>>()
        .join(" ");
    let header_line = if use_color {
        format!("\x1b[1m{header}\x1b[0m")
    } else {
        header.clone()
    };
    let _ = writeln!(output, "{padding}{header_line}");
    let rule = "-".repeat(header.len().min(max_width));
    let rule_line = if use_color {
        format!("\x1b[2m{rule}\x1b[0m")
    } else {
        rule
    };
    let _ = writeln!(output, "{padding}{rule_line}");
    for row in rows {
        if let Value::Object(map) = row {
            let line = keys
                .iter()
                .map(|key| {
                    let cell = map
                        .get(key)
                        .and_then(as_short_str)
                        .unwrap_or_else(|| "-".to_string());
                    format!(
                        "{:>width$}",
                        truncate(&cell, column_width),
                        width = column_width
                    )
                })
                .collect::<Vec<_>>()
                .join(" ");
            let _ = writeln!(output, "{padding}{line}");
        }
    }
}

fn format_script_record(value: &Value) -> String {
    let Some(map) = value.as_object() else {
        return escape_script_field(&value.to_string());
    };
    map.iter()
        .map(|(key, nested)| {
            let rendered = as_short_str(nested).unwrap_or_else(|| nested.to_string());
            format!(
                "{}={}",
                escape_script_field(key),
                escape_script_field(&rendered)
            )
        })
        .collect::<Vec<_>>()
        .join("\t")
}

fn escape_script_field(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '\t' => escaped.push_str("\\t"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '=' => escaped.push_str("\\="),
            other => escaped.push(other),
        }
    }
    escaped
}

fn union_object_keys(items: &[Value]) -> Vec<String> {
    let mut keys = Vec::new();
    for item in items {
        if let Some(map) = item.as_object() {
            for key in map.keys() {
                if !keys.iter().any(|existing| existing == key) {
                    keys.push(key.clone());
                }
            }
        }
    }
    keys
}

fn as_short_str(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(boolean) => Some(boolean.to_string()),
        Value::Null => Some("null".to_string()),
        _ => None,
    }
}

fn truncate(text: &str, max: usize) -> String {
    let flattened = text.replace('\n', " ");
    if flattened.chars().count() <= max {
        flattened
    } else {
        let end = flattened
            .char_indices()
            .nth(max.saturating_sub(1))
            .map(|(index, _)| index)
            .unwrap_or(flattened.len());
        format!("{}…", &flattened[..end])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_script_plain_one_record_per_line() {
        let value = serde_json::json!([
            {"id": 1, "name": "a"},
            {"id": 2, "name": "b"}
        ]);
        let output = format_script_plain(&value);
        assert_eq!(output.lines().count(), 2);
        assert!(output.contains("id=1"));
    }

    #[test]
    fn format_script_plain_escapes_delimiters_in_values() {
        let value = serde_json::json!([{"id": 1, "name": "a\tb\nc=d"}]);
        let output = format_script_plain(&value);
        assert_eq!(output.lines().count(), 1);
        assert!(output.contains("name=a\\tb\\nc\\=d"));
        assert_eq!(output.matches('\t').count(), 1);
        assert!(!output.contains('\n'));
    }

    #[test]
    fn human_plain_table_uses_color_when_enabled() {
        let value = serde_json::json!([{"id": 1, "name": "alpha"}]);
        let rendered = format_human_plain(&value, 80, true);
        assert!(rendered.contains("\x1b[1m"));
    }

    #[test]
    fn human_plain_table_omits_color_when_disabled() {
        let value = serde_json::json!([{"id": 1, "name": "alpha"}]);
        let rendered = format_human_plain(&value, 80, false);
        assert!(!rendered.contains("\x1b["));
    }
}
