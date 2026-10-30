use std::process::Output;

use anyhow::{Result, bail};

mod bitwarden;
mod keepass;
mod onepassword;
mod process;

pub(crate) use bitwarden::{
    bitwarden_custom_field, bitwarden_read, bitwarden_write, ensure_bitwarden_unlocked,
};
pub(crate) use keepass::{keepass_read, keepass_write};
pub(crate) use onepassword::{ensure_onepassword_session, onepassword_read, onepassword_write};
#[cfg(test)]
pub(crate) use onepassword::{parse_op_reference_item, upsert_onepassword_field};

pub(crate) const ONEPASSWORD_FIELD_DEFAULT: &str = "token";

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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
