use std::io::Write;
use std::process::{Command, Output};

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use tempfile::NamedTempFile;

use super::{ONEPASSWORD_FIELD_DEFAULT, SecretRef};

pub(crate) fn onepassword_read(source: &SecretRef, field: &Option<String>) -> Result<Output> {
    if let Some(reference) = &source.reference {
        return op_command(source)
            .args(["read", reference])
            .output()
            .context("failed to execute `op read`; is 1Password CLI installed and signed in?");
    }
    let item = source
        .item
        .as_ref()
        .context("1Password requires --reference or --item")?;
    let field = field
        .clone()
        .unwrap_or_else(|| ONEPASSWORD_FIELD_DEFAULT.to_string());
    op_command(source)
        .args(["item", "get", item, "--fields", &field])
        .output()
        .context("failed to execute `op item get`; is 1Password CLI installed and signed in?")
}

pub(crate) fn onepassword_write(target: &SecretRef, value: &str) -> Result<Output> {
    let field = target.field_or_default(ONEPASSWORD_FIELD_DEFAULT);

    if target.create {
        let title = target
            .title
            .clone()
            .or_else(|| target.item.clone())
            .context("1Password create requires --item or --title")?;
        let template = json!({
            "title": title,
            "category": "API_CREDENTIAL",
            "fields": [{
                "id": field,
                "type": "CONCEALED",
                "label": field,
                "value": value
            }]
        });
        return with_op_template(&template, |path| {
            op_command(target)
                .args(["item", "create", "--template", path])
                .output()
                .context(
                    "failed to execute `op item create`; is 1Password CLI installed and signed in?",
                )
        });
    }

    let item = resolve_onepassword_item(target)?;
    let template = onepassword_edit_template(&item, target, &field, value)?;
    with_op_template(&template, |path| {
        op_command(target)
            .args(["item", "edit", &item, "--template", path])
            .output()
            .context("failed to execute `op item edit`; is 1Password CLI installed and signed in?")
    })
}

pub(crate) fn onepassword_edit_template(
    item: &str,
    target: &SecretRef,
    field: &str,
    value: &str,
) -> Result<Value> {
    let output = op_command(target)
        .args(["item", "get", item, "--format", "json"])
        .output()
        .context("failed to fetch 1Password item for template edit")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("failed to read 1Password item '{item}': {stderr}");
    }
    let mut item_json: Value = serde_json::from_slice(&output.stdout)?;
    upsert_onepassword_field(&mut item_json, field, value);
    Ok(json!({ "fields": item_json.get("fields").cloned().unwrap_or(json!([])) }))
}

pub(crate) fn upsert_onepassword_field(item: &mut Value, field: &str, value: &str) {
    let fields = item
        .as_object_mut()
        .and_then(|obj| obj.get_mut("fields"))
        .and_then(Value::as_array_mut);
    if let Some(fields) = fields {
        if let Some(existing) = fields.iter_mut().find(|entry| {
            entry.get("label").and_then(Value::as_str) == Some(field)
                || entry.get("id").and_then(Value::as_str) == Some(field)
        }) {
            existing["value"] = json!(value);
            return;
        }
        fields.push(json!({
            "label": field,
            "type": "CONCEALED",
            "value": value
        }));
    }
}

pub(crate) fn with_op_template(
    value: &Value,
    run: impl FnOnce(&str) -> Result<Output>,
) -> Result<Output> {
    let mut file = NamedTempFile::new().context("failed to create temporary template file")?;
    serde_json::to_writer(&mut file, value).context("failed to write 1Password template")?;
    file.flush()?;
    let path = file.into_temp_path();
    let path_str = path
        .to_str()
        .context("temporary template path is not valid UTF-8")?;
    run(path_str)
}

pub(crate) fn op_session_active() -> bool {
    std::env::vars().any(|(name, _)| name.starts_with("OP_SESSION"))
}

pub(crate) fn ensure_onepassword_session(account: &Option<String>) -> Result<()> {
    if op_session_active() {
        return Ok(());
    }
    let mut command = Command::new("op");
    command.args(["signin", "--raw"]);
    if let Some(account) = account {
        command.args(["--account", account]);
    }
    let output = command
        .output()
        .context("failed to execute `op signin`; is 1Password CLI installed?")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "`op signin` failed: {stderr}. Sign in with `eval \"$(op signin)\"` or set OP_SESSION."
        );
    }
    Ok(())
}

pub(crate) fn op_command(source: &SecretRef) -> Command {
    let mut command = Command::new("op");
    if let Some(account) = &source.account {
        command.args(["--account", account]);
    }
    if let Some(vault) = &source.vault {
        command.args(["--vault", vault]);
    }
    if let Some(session) = op_session_from_env() {
        command.args(["--session", &session]);
    }
    command
}

pub(crate) fn op_session_from_env() -> Option<String> {
    if let Ok(session) = std::env::var("OP_SESSION")
        && !session.is_empty()
    {
        return Some(session);
    }
    std::env::vars()
        .find(|(name, _)| name.starts_with("OP_SESSION_"))
        .map(|(_, value)| value)
}
pub(crate) fn resolve_onepassword_item(target: &SecretRef) -> Result<String> {
    if let Some(item) = &target.item {
        return Ok(item.clone());
    }
    if let Some(reference) = &target.reference
        && let Some(item) = parse_op_reference_item(reference)
    {
        return Ok(item);
    }
    bail!("1Password write requires --item, --title (with --create), or an op:// reference")
}

pub(crate) fn parse_op_reference_item(reference: &str) -> Option<String> {
    let rest = reference.strip_prefix("op://")?;
    let mut parts = rest.split('/');
    let _vault = parts.next()?;
    parts.next().map(str::to_string)
}
