use anyhow::{Context, Result};
use keyring::Entry;
use std::path::PathBuf;

use crate::auth::StoredCredential;
use crate::config::timely_home;
use crate::credential_settings::{credential_backend, set_credential_backend};

pub const KEYRING_SERVICE: &str = "timely-cli";
const CREDENTIALS_DIR: &str = "credentials";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StorageBackend {
    #[default]
    Keyring,
    File,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CredentialLocation {
    None,
    Keyring,
    File,
    Env,
}

/// Human-readable OS credential backend (Cargo credential providers per platform).
pub fn keyring_backend_label() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "macOS Keychain"
    }
    #[cfg(target_os = "windows")]
    {
        "Windows Credential Manager"
    }
    #[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "openbsd"))]
    {
        "Secret Service (libsecret)"
    }
    #[cfg(not(any(
        target_os = "macos",
        target_os = "windows",
        target_os = "linux",
        target_os = "freebsd",
        target_os = "openbsd"
    )))]
    {
        "platform keyring"
    }
}

pub fn storage_backend(use_file: bool) -> StorageBackend {
    if use_file {
        StorageBackend::File
    } else {
        StorageBackend::Keyring
    }
}

pub fn store_profile_credential(
    profile: &str,
    credential: &StoredCredential,
    backend: StorageBackend,
) -> Result<()> {
    set_credential_backend(backend)?;
    match backend {
        StorageBackend::File => {
            save_credential_file(profile, credential)?;
            eprintln!(
                "Warning: credentials stored in plain file (opt-in). \
                 Omit --use-file to use OS keychain instead."
            );
            Ok(())
        }
        StorageBackend::Keyring => {
            store_keyring(profile, &serde_json::to_string(credential)?)?;
            let _ = delete_credential_file(profile);
            Ok(())
        }
    }
}

pub fn load_profile_credential(profile: &str) -> Result<Option<StoredCredential>> {
    match credential_backend()? {
        StorageBackend::File => load_credential_file(profile),
        StorageBackend::Keyring => load_profile_keyring(profile),
    }
}

fn load_profile_keyring(profile: &str) -> Result<Option<StoredCredential>> {
    if let Some(raw) = load_keyring(profile)? {
        return Ok(Some(parse_credential(&raw)?));
    }
    if credential_file_path(profile)?.exists() {
        let path = credential_file_path(profile)?;
        let credential = load_credential_file(profile)?
            .context("Plain credential file exists but could not be parsed for keychain migration")?;
        store_keyring(profile, &serde_json::to_string(&credential)?).with_context(|| {
            format!(
                "Plain credential file at {} found but OS keychain is unavailable. \
                 Run `timely auth token` to store in keychain, or opt in with `--use-file`.",
                path.display()
            )
        })?;
        delete_credential_file(profile)?;
        set_credential_backend(StorageBackend::Keyring)?;
        eprintln!("Migrated credentials from plain file to OS keychain.");
        return Ok(Some(credential));
    }
    Ok(None)
}

pub fn delete_profile_credential(profile: &str) -> Result<()> {
    let _ = delete_keyring(profile);
    delete_credential_file(profile)
}

pub fn profile_credential_location(profile: &str) -> Result<CredentialLocation> {
    if std::env::var("TIMELY_TOKEN").is_ok() {
        return Ok(CredentialLocation::Env);
    }
    match credential_backend()? {
        StorageBackend::File => {
            if credential_file_path(profile)?.exists() {
                Ok(CredentialLocation::File)
            } else {
                Ok(CredentialLocation::None)
            }
        }
        StorageBackend::Keyring => {
            if load_keyring(profile)?.is_some() {
                Ok(CredentialLocation::Keyring)
            } else if credential_file_path(profile)?.exists() {
                Ok(CredentialLocation::File)
            } else {
                Ok(CredentialLocation::None)
            }
        }
    }
}

pub fn credential_file_path(profile: &str) -> Result<PathBuf> {
    let home = timely_home().ok_or_else(|| anyhow::anyhow!("could not resolve timely home"))?;
    Ok(home.join(CREDENTIALS_DIR).join(format!("{profile}.json")))
}

fn parse_credential(raw: &str) -> Result<StoredCredential> {
    serde_json::from_str(raw).context("Invalid credential payload in credential store")
}

fn save_credential_file(profile: &str, credential: &StoredCredential) -> Result<()> {
    let path = credential_file_path(profile)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_vec_pretty(credential)?)?;
    Ok(())
}

fn load_credential_file(profile: &str) -> Result<Option<StoredCredential>> {
    let path = credential_file_path(profile)?;
    if !path.exists() {
        return Ok(None);
    }
    Ok(Some(
        serde_json::from_slice(&std::fs::read(path)?).context("Invalid credential file")?,
    ))
}

fn delete_credential_file(profile: &str) -> Result<()> {
    let path = credential_file_path(profile)?;
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

fn keyring_entry(profile: &str) -> Result<Entry> {
    Entry::new(KEYRING_SERVICE, profile).context("failed to open OS keyring entry")
}

fn store_keyring(profile: &str, value: &str) -> Result<()> {
    keyring_entry(profile)?
        .set_password(value)
        .context("failed to store credential in OS keyring")
}

fn load_keyring(profile: &str) -> Result<Option<String>> {
    let entry = match keyring_entry(profile) {
        Ok(entry) => entry,
        Err(_) => return Ok(None),
    };
    match entry.get_password() {
        Ok(value) => Ok(Some(value)),
        Err(keyring::Error::NoEntry)
        | Err(keyring::Error::NoStorageAccess(_))
        | Err(keyring::Error::PlatformFailure(_)) => Ok(None),
        Err(err) => Err(err).context("failed to read credential from OS keyring"),
    }
}

fn delete_keyring(profile: &str) -> Result<()> {
    let entry = match keyring_entry(profile) {
        Ok(entry) => entry,
        Err(_) => return Ok(()),
    };
    match entry.delete_credential() {
        Ok(())
        | Err(keyring::Error::NoEntry)
        | Err(keyring::Error::NoStorageAccess(_))
        | Err(keyring::Error::PlatformFailure(_)) => Ok(()),
        Err(err) => Err(err).context("failed to delete credential from OS keyring"),
    }
}

pub fn location_label(location: CredentialLocation) -> &'static str {
    match location {
        CredentialLocation::None => "not configured",
        CredentialLocation::Keyring => "OS keychain",
        CredentialLocation::File => "plain credential file (opt-in)",
        CredentialLocation::Env => "TIMELY_TOKEN env",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credential_file_path_is_under_timely_home() {
        let path = credential_file_path("work").unwrap();
        assert!(path.ends_with("credentials/work.json"));
    }

    #[test]
    fn storage_backend_selects_file_only_when_opted_in() {
        assert_eq!(storage_backend(false), StorageBackend::Keyring);
        assert_eq!(storage_backend(true), StorageBackend::File);
    }
}
