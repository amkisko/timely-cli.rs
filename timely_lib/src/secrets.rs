use std::process::Command;

use anyhow::{Context, Result, anyhow, bail};
use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecretProvider {
    Onepassword,
    Bitwarden,
    Keepass,
}

#[derive(Clone, Debug)]
pub struct SecretSource {
    pub provider: SecretProvider,
    pub reference: Option<String>,
    pub item: Option<String>,
    pub field: Option<String>,
    pub database: Option<String>,
}

pub fn fetch_secret(source: SecretSource) -> Result<String> {
    let requested_field = source.field.clone();
    let output = match source.provider {
        SecretProvider::Onepassword => onepassword(&source, &requested_field)?,
        SecretProvider::Bitwarden => bitwarden(&source, &requested_field)?,
        SecretProvider::Keepass => keepass(&source, &requested_field)?,
    };

    if !output.status.success() {
        let code = output.status.code().unwrap_or(-1);
        bail!("secret command failed with exit status {code}");
    }
    let mut token = String::from_utf8(output.stdout)?.trim().to_string();
    if source.provider == SecretProvider::Bitwarden
        && requested_field
            .as_deref()
            .is_some_and(|field| field != "password")
    {
        token = bitwarden_custom_field(&token, &requested_field.unwrap())?;
    }
    if token.is_empty() {
        bail!("secret command returned an empty token");
    }
    Ok(token)
}

fn onepassword(source: &SecretSource, field: &Option<String>) -> Result<std::process::Output> {
    if let Some(reference) = &source.reference {
        return Command::new("op")
            .args(["read", reference])
            .output()
            .context("failed to execute `op read`; is 1Password CLI installed and signed in?");
    }
    let item = source
        .item
        .as_ref()
        .context("1Password requires --reference or --item")?;
    let field = field.clone().unwrap_or_else(|| "token".to_string());
    Command::new("op")
        .args(["item", "get", item, "--fields", &field])
        .output()
        .context("failed to execute `op item get`; is 1Password CLI installed and signed in?")
}

fn bitwarden(source: &SecretSource, field: &Option<String>) -> Result<std::process::Output> {
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

fn keepass(source: &SecretSource, field: &Option<String>) -> Result<std::process::Output> {
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
}
