//! Dotenv-style parse and rewrite helpers for home config files.

use std::collections::HashMap;

use crate::config_keys::is_allowed_config_key;

/// Parse `KEY=VALUE` lines from a dotenv-style config file.
pub fn parse_config_content(content: &str) -> HashMap<String, String> {
    let mut entries = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = parse_config_line(line)
            && is_allowed_config_key(&key)
        {
            entries.insert(key, value);
        }
    }
    entries
}

pub fn parse_config_line(line: &str) -> Option<(String, String)> {
    let (key, rest) = line.split_once('=')?;
    let key = key.trim();
    if key.is_empty() {
        return None;
    }
    Some((key.to_string(), parse_config_value(rest.trim())))
}

pub fn upsert_config_line(content: &str, key: &str, value: &str) -> String {
    let assignment = format_config_assignment(key, value);
    let mut found = false;
    let mut lines: Vec<String> = if content.is_empty() {
        Vec::new()
    } else {
        content.lines().map(String::from).collect()
    };

    for line in &mut lines {
        if let Some((existing_key, _)) = parse_config_line(line)
            && existing_key == key
        {
            *line = assignment.clone();
            found = true;
            break;
        }
    }

    if !found {
        if !lines.is_empty() && lines.last().is_some_and(|line| !line.is_empty()) {
            lines.push(String::new());
        }
        lines.push(assignment);
    }

    join_lines(&lines)
}

pub fn remove_config_line(content: &str, key: &str) -> String {
    let lines: Vec<String> = content
        .lines()
        .filter(|line| parse_config_line(line).is_none_or(|(existing_key, _)| existing_key != key))
        .map(String::from)
        .collect();
    join_lines(&lines)
}

fn parse_config_value(raw: &str) -> String {
    if raw.len() >= 2 {
        let bytes = raw.as_bytes();
        let quote = bytes[0];
        if (quote == b'"' || quote == b'\'') && bytes[raw.len() - 1] == quote {
            return raw[1..raw.len() - 1].to_string();
        }
    }
    raw.to_string()
}

fn format_config_assignment(key: &str, value: &str) -> String {
    if value.chars().any(char::is_whitespace) {
        format!("{key}=\"{value}\"")
    } else {
        format!("{key}={value}")
    }
}

fn join_lines(lines: &[String]) -> String {
    if lines.is_empty() {
        return String::new();
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_reads_comments_quotes_and_skips_secrets() {
        let content = r#"
# oauth
TIMELY_CLIENT_ID=public-client
TIMELY_OUTPUT="json"
TIMELY_TOKEN=secret-not-allowed
"#;
        let entries = parse_config_content(content);
        assert_eq!(
            entries.get("TIMELY_CLIENT_ID").map(String::as_str),
            Some("public-client")
        );
        assert_eq!(
            entries.get("TIMELY_OUTPUT").map(String::as_str),
            Some("json")
        );
        assert!(!entries.contains_key("TIMELY_TOKEN"));
    }

    #[test]
    fn upsert_and_remove_config_lines() {
        let updated = upsert_config_line("", "TIMELY_CLIENT_ID", "abc");
        assert_eq!(updated, "TIMELY_CLIENT_ID=abc\n");
        let updated = upsert_config_line(
            "# config\nTIMELY_CLIENT_ID=old\n",
            "TIMELY_CLIENT_ID",
            "new-id",
        );
        assert!(updated.contains("TIMELY_CLIENT_ID=new-id"));
        assert!(!updated.contains("old"));
        let removed = remove_config_line(&updated, "TIMELY_CLIENT_ID");
        assert!(!removed.contains("TIMELY_CLIENT_ID"));
    }
}
