use anyhow::{bail, Result};
use serde_json::Value;
use std::process::Command;

use crate::cli::{
    AuthCommand, AuthRunWithArgs, AuthSinkArgs, AuthSourceArgs, AuthSubcommand, AuthTokenArgs,
    BitwardenSecretArgs, KeepassSecretArgs, OauthCommand, OnepasswordSecretArgs,
    SecretProviderCommand,
};
use crate::export_io;
use crate::oauth::run_oauth_flow;
use timely_lib::{
    fetch_secret, provider_label, storage_backend, store_secret, Api, SecretProvider as LibSecretProvider,
    SecretRef, StoredCredential,
};
use timely_lib::{auth_export_value, auth_status_value};
use timely_lib::util::{read_secret_value, token_fingerprint};

pub async fn run_auth(api: &Api, cmd: AuthCommand) -> Result<Option<Value>> {
    match cmd.command {
        AuthSubcommand::Token(args) => {
            run_token(api, args).await?;
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
            println!(
                "Removed credentials for profile '{}' from keychain and credential file.",
                api.profile
            );
            Ok(None)
        }
        AuthSubcommand::Source(args) => {
            run_source(api, args).await?;
            Ok(None)
        }
        AuthSubcommand::Sink(args) => {
            run_sink(api, args)?;
            Ok(None)
        }
        AuthSubcommand::RunWith(args) => {
            run_with(api, args)?;
            Ok(None)
        }
        AuthSubcommand::Oauth(oauth) => {
            run_oauth(api, oauth).await?;
            Ok(None)
        }
    }
}

async fn run_token(api: &Api, args: AuthTokenArgs) -> Result<()> {
    let token = resolve_token_input(args.token, args.token_file)?;
    let credential = api
        .prepare_credential(StoredCredential::bearer(token))
        .await?;
    store_credential(api, credential, "Stored token", args.use_file)?;
    Ok(())
}

async fn run_source(api: &Api, args: AuthSourceArgs) -> Result<()> {
    let secret = secret_ref_from_command(args.provider)?;
    let token = fetch_secret(&secret)?;
    println!("Fetched token ({})", token_fingerprint(&token));
    if args.store {
        let credential = api
            .prepare_credential(StoredCredential::bearer(token))
            .await?;
        store_credential(api, credential, "Stored token", args.use_file)?;
    } else {
        println!("{token}");
    }
    Ok(())
}

fn run_sink(api: &Api, args: AuthSinkArgs) -> Result<()> {
    let token = resolve_sink_token(api, args.token, args.token_file, args.from_profile)?;
    let secret = secret_ref_from_command(args.provider)?;
    store_secret(&secret, &token)?;
    println!(
        "Wrote token ({}) to {}.",
        token_fingerprint(&token),
        provider_label(secret.provider)
    );
    Ok(())
}

fn run_with(api: &Api, args: AuthRunWithArgs) -> Result<()> {
    if args.command.is_empty() {
        bail!("provide a command after `--`");
    }
    let credential = api.load_credential()?.ok_or_else(|| {
        anyhow::anyhow!("no stored credentials for profile '{}'", api.profile)
    })?;
    let mut command = Command::new(&args.command[0]);
    command
        .args(&args.command[1..])
        .env("TIMELY_TOKEN", credential.access_token);
    let status = command.status()?;
    if !status.success() {
        bail!("command exited with status {}", status.code().unwrap_or(-1));
    }
    Ok(())
}

async fn run_oauth(api: &Api, oauth: OauthCommand) -> Result<()> {
    let client_secret = oauth.resolved_client_secret()?;
    let mut credential = run_oauth_flow(api, &oauth, client_secret.clone()).await?;
    credential.oauth_client_id = Some(oauth.client_id);
    credential.oauth_client_secret = client_secret;
    let credential = api.prepare_credential(credential).await?;
    store_credential(
        api,
        credential,
        "Stored OAuth credentials",
        oauth.use_file,
    )?;
    Ok(())
}

pub fn auth_command_value(api: &Api, cmd: AuthCommand) -> Result<Value> {
    match cmd.command {
        AuthSubcommand::Status => auth_status_value(api),
        AuthSubcommand::Export { .. } => auth_export_value(api),
        AuthSubcommand::Token(_)
        | AuthSubcommand::Logout
        | AuthSubcommand::Source(_)
        | AuthSubcommand::Sink(_)
        | AuthSubcommand::RunWith(_)
        | AuthSubcommand::Oauth(_) => Err(anyhow::anyhow!(
            "auth token/logout/oauth/source/sink/run-with cannot run inside batch (state-changing)"
        )),
    }
}

fn resolve_token_input(token: Option<String>, token_file: Option<String>) -> Result<String> {
    match (&token, &token_file) {
        (None, None) => bail!("provide --token, --token-file, or TIMELY_TOKEN"),
        _ => read_secret_value(token, token_file),
    }
}

fn resolve_sink_token(
    api: &Api,
    token: Option<String>,
    token_file: Option<String>,
    from_profile: bool,
) -> Result<String> {
    if from_profile {
        return api
            .load_credential()?
            .map(|credential| credential.access_token)
            .ok_or_else(|| {
                anyhow::anyhow!("no stored credentials for profile '{}'", api.profile)
            });
    }
    resolve_token_input(token, token_file)
}

fn store_credential(
    api: &Api,
    credential: StoredCredential,
    label: &str,
    use_file: bool,
) -> Result<()> {
    let account_id = credential.account_id;
    let refresh = credential.refresh_token.is_some();
    api.store_credential_with_backend(credential, storage_backend(use_file))?;
    let backend_label = if use_file {
        "plain credential file (opt-in)"
    } else {
        "OS keychain"
    };
    println!("{label} for profile '{}' in {backend_label}.", api.profile);
    if let Some(account_id) = account_id {
        println!("Default account: {account_id}");
        if let Some(path) = timely_lib::runtime_env::set("TIMELY_ACCOUNT_ID", &account_id.to_string())? {
            println!("Stored TIMELY_ACCOUNT_ID in {path}.");
        }
    }
    if refresh {
        println!("Refresh token: available");
    }
    Ok(())
}

fn secret_ref_from_command(provider: SecretProviderCommand) -> Result<SecretRef> {
    Ok(match provider {
        SecretProviderCommand::Onepassword(args) => onepassword_ref(&args),
        SecretProviderCommand::Bitwarden(args) => bitwarden_ref(&args),
        SecretProviderCommand::Keepass(args) => keepass_ref(&args),
    })
}

fn onepassword_ref(args: &OnepasswordSecretArgs) -> SecretRef {
    SecretRef {
        provider: LibSecretProvider::Onepassword,
        account: args.account.clone(),
        reference: args.reference.clone(),
        item: args.item.clone(),
        field: args.field.clone(),
        database: None,
        vault: args.vault.clone(),
        title: args.title.clone(),
        create: args.create,
    }
}

fn bitwarden_ref(args: &BitwardenSecretArgs) -> SecretRef {
    SecretRef {
        provider: LibSecretProvider::Bitwarden,
        account: None,
        reference: None,
        item: args.item.clone(),
        field: args.field.clone(),
        database: None,
        vault: None,
        title: args.title.clone(),
        create: args.create,
    }
}

fn keepass_ref(args: &KeepassSecretArgs) -> SecretRef {
    SecretRef {
        provider: LibSecretProvider::Keepass,
        account: None,
        reference: None,
        item: args.item.clone(),
        field: args.field.clone(),
        database: args.database.clone(),
        vault: None,
        title: args.title.clone(),
        create: args.create,
    }
}
