use std::process::{Command, Output, Stdio};

use anyhow::{Context, Result, anyhow, bail};
use serde_json::{Value, json};

use super::SecretRef;
use super::process::write_stdin;

pub(crate) fn ensure_bitwarden_unlocked() -> Result<()> {
    let output = Command::new("bw")
        .args(["status"])
        .output()
        .context("failed to execute `bw status`; is Bitwarden CLI installed?")?;
    if !output.status.success() {
        bail!("`bw status` failed; run `bw login` and `bw unlock` first");
    }
    let status: Value = serde_json::from_slice(&output.stdout)?;
    match status.get("status").and_then(Value::as_str) {
        Some("unlocked") => Ok(()),
        Some("locked") => bail!("Bitwarden vault is locked; run `bw unlock` first"),
        Some(other) => bail!("Bitwarden vault status is '{other}'; run `bw login` and `bw unlock`"),
        None => bail!("Bitwarden vault status unavailable; run `bw login` and `bw unlock`"),
    }
}
pub(crate) fn bitwarden_read(source: &SecretRef, field: &Option<String>) -> Result<Output> {
    let item = source.item.as_ref().context("Bitwarden requires --item")?;
    let field = field.clone().unwrap_or_else(|| "password".to_string());
    if field == "password" {
        return Command::new("bw")
            .args(["get", "password", item])
            .output()
            .context(
                "failed to execute `bw get password`; is Bitwarden CLI installed and unlocked?",
            );
    }
    Command::new("bw")
        .args(["get", "item", item])
        .output()
        .context("failed to execute `bw get item`; is Bitwarden CLI installed and unlocked?")
}

pub(crate) fn bitwarden_write(target: &SecretRef, value: &str) -> Result<Output> {
    let item_name = target.item.as_ref().context("Bitwarden requires --item")?;
    let field = target.field_or_default("password");

    if target.create {
        let title = target.title.clone().unwrap_or_else(|| item_name.clone());
        let template = json!({
            "type": 1,
            "name": title,
            "login": {
                "password": if field == "password" { value } else { "" }
            },
            "fields": if field == "password" {
                Value::Array(vec![])
            } else {
                json!([{ "name": field, "value": value, "type": 0 }])
            }
        });
        let encoded = bw_encode(template.to_string())?;
        return Command::new("bw")
            .args(["create", "item", &encoded])
            .output()
            .context(
                "failed to execute `bw create item`; is Bitwarden CLI installed and unlocked?",
            );
    }

    let item_id = bitwarden_item_id(item_name)?;
    let current = Command::new("bw")
        .args(["get", "item", &item_id])
        .output()
        .context("failed to execute `bw get item`")?;
    if !current.status.success() {
        bail!("Bitwarden item not found: {item_name}");
    }
    let mut item: Value = serde_json::from_slice(&current.stdout)?;
    if field == "password" {
        item["login"]["password"] = json!(value);
    } else {
        upsert_bitwarden_field(&mut item, &field, value);
    }
    let encoded = bw_encode(item.to_string())?;
    Command::new("bw")
        .args(["edit", "item", &item_id, &encoded])
        .output()
        .context("failed to execute `bw edit item`; is Bitwarden CLI installed and unlocked?")
}

pub(crate) fn bw_encode(raw_json: String) -> Result<String> {
    let output = Command::new("bw")
        .arg("encode")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .context("failed to execute `bw encode`")?;
    let output = write_stdin(output, raw_json.as_bytes())?;
    if !output.status.success() {
        bail!("`bw encode` failed");
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

pub(crate) fn bitwarden_item_id(item_name: &str) -> Result<String> {
    let list = Command::new("bw")
        .args(["list", "items", "--search", item_name])
        .output()
        .context("failed to execute `bw list items`")?;
    if !list.status.success() {
        bail!("failed to search Bitwarden vault for item '{item_name}'");
    }
    let items: Vec<Value> = serde_json::from_slice(&list.stdout)?;
    items
        .into_iter()
        .find(|item| item.get("name").and_then(Value::as_str) == Some(item_name))
        .and_then(|item| item.get("id").and_then(Value::as_str).map(str::to_string))
        .ok_or_else(|| anyhow!("Bitwarden item not found: {item_name}"))
}

pub(crate) fn upsert_bitwarden_field(item: &mut Value, field: &str, value: &str) {
    let fields = item.as_object_mut().and_then(|obj| {
        obj.entry("fields")
            .or_insert_with(|| json!([]))
            .as_array_mut()
    });
    if let Some(fields) = fields {
        if let Some(existing) = fields
            .iter_mut()
            .find(|entry| entry.get("name").and_then(Value::as_str) == Some(field))
        {
            existing["value"] = json!(value);
            return;
        }
        fields.push(json!({ "name": field, "value": value, "type": 0 }));
    }
}
pub(crate) fn bitwarden_custom_field(item_json: &str, field: &str) -> Result<String> {
    let item: Value = serde_json::from_str(item_json)?;
    item.get("fields")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|field_value| field_value.get("name").and_then(Value::as_str) == Some(field))
        .and_then(|field_value| field_value.get("value").and_then(Value::as_str))
        .map(str::to_string)
        .ok_or_else(|| anyhow!("Bitwarden custom field '{field}' not found"))
}
