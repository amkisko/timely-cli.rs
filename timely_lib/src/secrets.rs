use std::io::Write;
use std::process::{Command, Output, Stdio};

use anyhow::{anyhow, bail, Context, Result};
use serde_json::{json, Value};
use tempfile::NamedTempFile;

const ONEPASSWORD_FIELD_DEFAULT: &str = "token";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecretProvider {
    Onepassword,
    Bitwarden,
    Keepass,
}

#[derive(Clone, Debug)]
pub struct SecretRef {
    pub provider: SecretProvider,
    pub account: Option<String>,
    pub reference: Option<String>,
    pub item: Option<String>,
    pub field: Option<String>,
    pub database: Option<String>,
    pub vault: Option<String>,
    pub title: Option<String>,
    pub create: bool,
}

impl SecretRef {
    pub fn field_or_default(&self, provider_default: &str) -> String {
        self.field
            .clone()
            .unwrap_or_else(|| provider_default.to_string())
    }
}

pub fn provider_label(provider: SecretProvider) -> &'static str {
    match provider {
        SecretProvider::Onepassword => "1Password",
        SecretProvider::Bitwarden => "Bitwarden",
        SecretProvider::Keepass => "KeePassXC",
    }
}

pub fn fetch_secret(source: &SecretRef) -> Result<String> {
    let requested_field = source.field.clone();
    match source.provider {
        SecretProvider::Onepassword => ensure_onepassword_session(&source.account)?,
        SecretProvider::Bitwarden => ensure_bitwarden_unlocked()?,
        SecretProvider::Keepass => {}
    }
    let output = match source.provider {
        SecretProvider::Onepassword => onepassword_read(source, &requested_field)?,
        SecretProvider::Bitwarden => bitwarden_read(source, &requested_field)?,
        SecretProvider::Keepass => keepass_read(source, &requested_field)?,
    };

    decode_fetch_output(source.provider, &requested_field, output)
}

pub fn store_secret(target: &SecretRef, value: &str) -> Result<()> {
    if value.is_empty() {
        bail!("secret value is empty");
    }
    match target.provider {
        SecretProvider::Onepassword => ensure_onepassword_session(&target.account)?,
        SecretProvider::Bitwarden => ensure_bitwarden_unlocked()?,
        SecretProvider::Keepass => {}
    }
    let output = match target.provider {
        SecretProvider::Onepassword => onepassword_write(target, value)?,
        SecretProvider::Bitwarden => bitwarden_write(target, value)?,
        SecretProvider::Keepass => keepass_write(target, value)?,
    };
    if !output.status.success() {
        let code = output.status.code().unwrap_or(-1);
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("secret write command failed with exit status {code}: {stderr}");
    }
    Ok(())
}

fn decode_fetch_output(
    provider: SecretProvider,
    requested_field: &Option<String>,
    output: Output,
) -> Result<String> {
    if !output.status.success() {
        let code = output.status.code().unwrap_or(-1);
        bail!("secret read command failed with exit status {code}");
    }
    let mut token = String::from_utf8(output.stdout)?.trim().to_string();
    if provider == SecretProvider::Bitwarden
        && requested_field
            .as_deref()
            .is_some_and(|field| field != "password")
    {
        token = bitwarden_custom_field(&token, &requested_field.clone().unwrap())?;
    }
    if token.is_empty() {
        bail!("secret command returned an empty token");
    }
    Ok(token)
}

fn onepassword_read(source: &SecretRef, field: &Option<String>) -> Result<Output> {
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

fn onepassword_write(target: &SecretRef, value: &str) -> Result<Output> {
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
            .context(
                "failed to execute `op item edit`; is 1Password CLI installed and signed in?",
            )
    })
}

fn onepassword_edit_template(
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

fn upsert_onepassword_field(item: &mut Value, field: &str, value: &str) {
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

fn with_op_template(value: &Value, run: impl FnOnce(&str) -> Result<Output>) -> Result<Output> {
    let mut file = NamedTempFile::new().context("failed to create temporary template file")?;
    serde_json::to_writer(&mut file, value).context("failed to write 1Password template")?;
    file.flush()?;
    let path = file.into_temp_path();
    let path_str = path
        .to_str()
        .context("temporary template path is not valid UTF-8")?;
    run(path_str)
}

fn op_session_active() -> bool {
    std::env::vars().any(|(name, _)| name.starts_with("OP_SESSION"))
}

fn ensure_onepassword_session(account: &Option<String>) -> Result<()> {
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

fn op_command(source: &SecretRef) -> Command {
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

fn op_session_from_env() -> Option<String> {
    if let Ok(session) = std::env::var("OP_SESSION")
        && !session.is_empty()
    {
        return Some(session);
    }
    std::env::vars()
        .find(|(name, _)| name.starts_with("OP_SESSION_"))
        .map(|(_, value)| value)
}

fn ensure_bitwarden_unlocked() -> Result<()> {
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

fn resolve_onepassword_item(target: &SecretRef) -> Result<String> {
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

fn parse_op_reference_item(reference: &str) -> Option<String> {
    let rest = reference.strip_prefix("op://")?;
    let mut parts = rest.split('/');
    let _vault = parts.next()?;
    parts.next().map(str::to_string)
}

fn bitwarden_read(source: &SecretRef, field: &Option<String>) -> Result<Output> {
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

fn bitwarden_write(target: &SecretRef, value: &str) -> Result<Output> {
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

fn bw_encode(raw_json: String) -> Result<String> {
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

fn bitwarden_item_id(item_name: &str) -> Result<String> {
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

fn upsert_bitwarden_field(item: &mut Value, field: &str, value: &str) {
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

fn keepass_read(source: &SecretRef, field: &Option<String>) -> Result<Output> {
    let database = source
        .database
        .as_ref()
        .context("KeePassXC requires --database")?;
    let item = source.item.as_ref().context("KeePassXC requires --item")?;
    let field = field.clone().unwrap_or_else(|| "Password".to_string());
    Command::new("keepassxc-cli")
        .args(["show", "-q", "-a", &field, database, item])
        .output()
        .context("failed to execute `keepassxc-cli show`; is KeePassXC CLI installed?")
}

fn keepass_write(target: &SecretRef, value: &str) -> Result<Output> {
    let database = target
        .database
        .as_ref()
        .context("KeePassXC requires --database")?;
    let entry = target.item.as_ref().context("KeePassXC requires --item")?;
    let field = target.field_or_default("Password");

    if target.create {
        let title = target
            .title
            .clone()
            .or_else(|| entry.rsplit('/').next().map(str::to_string))
            .unwrap_or_else(|| entry.clone());
        let mut command = Command::new("keepassxc-cli");
        command.args(["add", "-q", database, entry, "--title", &title]);
        if field == "Password" {
            return keepass_password_prompt(&mut command, value);
        }
        command.args(["--notes", &format!("{field}: {value}")]);
        return command
            .output()
            .context("failed to execute `keepassxc-cli add`; is KeePassXC CLI installed?");
    }

    if field == "Password" {
        let mut command = Command::new("keepassxc-cli");
        command.args(["edit", "-q", database, entry, "-p"]);
        return keepass_password_prompt(&mut command, value);
    }

    Command::new("keepassxc-cli")
        .args(["edit", "-q", database, entry, "--notes", value])
        .output()
        .context("failed to execute `keepassxc-cli edit`; is KeePassXC CLI installed?")
}

fn keepass_password_prompt(command: &mut Command, value: &str) -> Result<Output> {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to spawn KeePassXC CLI password prompt")?;
    {
        let stdin = child
            .stdin
            .as_mut()
            .context("KeePassXC CLI stdin unavailable")?;
        stdin.write_all(format!("{value}\n").as_bytes())?;
    }
    child
        .wait_with_output()
        .context("failed waiting for KeePassXC CLI")
}

fn write_stdin(mut child: std::process::Child, bytes: &[u8]) -> Result<Output> {
    {
        let stdin = child
            .stdin
            .as_mut()
            .context("subprocess stdin unavailable")?;
        stdin.write_all(bytes)?;
    }
    child
        .wait_with_output()
        .context("failed waiting for subprocess")
}

fn bitwarden_custom_field(item_json: &str, field: &str) -> Result<String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_bitwarden_custom_field() {
        let item = r#"{"fields":[{"name":"token","value":"secret"}]}"#;
        assert_eq!(bitwarden_custom_field(item, "token").unwrap(), "secret");
    }

    #[test]
    fn parses_op_reference_item() {
        assert_eq!(
            parse_op_reference_item("op://Vault/Timely/token"),
            Some("Timely".to_string())
        );
    }

    #[test]
    fn upserts_onepassword_custom_field() {
        let mut item = json!({"fields":[{"label":"token","value":"old"}]});
        upsert_onepassword_field(&mut item, "token", "new");
        assert_eq!(item["fields"][0]["value"].as_str(), Some("new"));
    }
}
