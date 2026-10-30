use std::io::Write;
use std::process::{Command, Output, Stdio};

use anyhow::{Context, Result};

use super::SecretRef;

pub(crate) fn keepass_read(source: &SecretRef, field: &Option<String>) -> Result<Output> {
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

pub(crate) fn keepass_write(target: &SecretRef, value: &str) -> Result<Output> {
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

pub(crate) fn keepass_password_prompt(command: &mut Command, value: &str) -> Result<Output> {
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
