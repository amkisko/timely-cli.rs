use anyhow::{Result, bail};
use serde_json::Value;

use crate::cli::{AuthCommand, AuthSubcommand, SecretProvider, SourceCommand};
use crate::export_io;
use crate::oauth::run_oauth_flow;
use timely_lib::{
    Api, StoredCredential, auth_export_value, auth_status_value, runtime_env,
    secrets::{SecretProvider as LibSecretProvider, SecretSource, fetch_secret},
    util::{read_secret_value, token_fingerprint},
};

pub async fn run_auth(api: &Api, cmd: AuthCommand) -> Result<Option<Value>> {
    match cmd.command {
        AuthSubcommand::Token { token, token_file } => {
            let token = resolve_token(token, token_file)?;
            let credential = api
                .prepare_credential(StoredCredential::bearer(token))
                .await?;
            store_credential(api, credential, "Stored token")?;
            Ok(None)
        }
        AuthSubcommand::Status => Ok(Some(auth_status_value(api)?)),
        AuthSubcommand::Export { file } => {
            let value = auth_export_value(api)?;
            export_io::write_json_output(&value, file.as_deref())?;
            Ok(None)
        }
        AuthSubcommand::Logout => {
            api.delete_credential()?;
            println!("Removed credentials for profile '{}'.", api.profile);
            Ok(None)
        }
        AuthSubcommand::Source(source) => {
            let token = fetch_secret(secret_source(source))?;
            println!("Fetched token ({})", token_fingerprint(&token));
            let credential = api
                .prepare_credential(StoredCredential::bearer(token))
                .await?;
            store_credential(api, credential, "Stored token")?;
            Ok(None)
        }
        AuthSubcommand::Oauth(oauth) => {
            let client_secret = oauth.resolved_client_secret()?;
            let mut credential = run_oauth_flow(api, &oauth, client_secret.clone()).await?;
            credential.oauth_client_id = Some(oauth.client_id);
            credential.oauth_client_secret = client_secret;
            let credential = api.prepare_credential(credential).await?;
            store_credential(api, credential, "Stored OAuth credentials")?;
            Ok(None)
        }
    }
}

fn resolve_token(token: Option<String>, token_file: Option<String>) -> Result<String> {
    match (&token, &token_file) {
        (None, None) => bail!("provide --token, --token-file, or TIMELY_TOKEN"),
        _ => read_secret_value(token, token_file),
    }
}

pub fn auth_command_value(api: &Api, cmd: AuthCommand) -> Result<Value> {
    match cmd.command {
        AuthSubcommand::Status => auth_status_value(api),
        AuthSubcommand::Export { .. } => auth_export_value(api),
        AuthSubcommand::Token { .. }
        | AuthSubcommand::Logout
        | AuthSubcommand::Source(_)
        | AuthSubcommand::Oauth(_) => Err(anyhow::anyhow!(
            "auth token/logout/oauth/source cannot run inside batch (state-changing)"
        )),
    }
}

fn store_credential(api: &Api, credential: StoredCredential, label: &str) -> Result<()> {
    let account_id = credential.account_id;
    let refresh = credential.refresh_token.is_some();
    api.store_credential(credential)?;
    println!("{label} for profile '{}'.", api.profile);
    if let Some(account_id) = account_id {
        println!("Default account: {account_id}");
        if let Some(path) = runtime_env::set("TIMELY_ACCOUNT_ID", &account_id.to_string())? {
            println!("Stored TIMELY_ACCOUNT_ID in {path}.");
        }
    }
    if refresh {
        println!("Refresh token: available");
    }
    Ok(())
}

fn secret_source(source: SourceCommand) -> SecretSource {
    SecretSource {
        provider: match source.provider {
            SecretProvider::Onepassword => LibSecretProvider::Onepassword,
            SecretProvider::Bitwarden => LibSecretProvider::Bitwarden,
            SecretProvider::Keepass => LibSecretProvider::Keepass,
        },
        reference: source.reference,
        item: source.item,
        field: source.field,
        database: source.database,
    }
}
