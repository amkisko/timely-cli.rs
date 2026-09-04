use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::config::timely_home;
use crate::credential_store::StorageBackend;

const SETTINGS_FILE: &str = "settings.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialSettings {
    #[serde(default)]
    pub credential_backend: StorageBackendSetting,
}

impl Default for CredentialSettings {
    fn default() -> Self {
        Self {
            credential_backend: StorageBackendSetting::Keyring,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StorageBackendSetting {
    #[default]
    Keyring,
    File,
}

impl From<StorageBackendSetting> for StorageBackend {
    fn from(value: StorageBackendSetting) -> Self {
        match value {
            StorageBackendSetting::Keyring => StorageBackend::Keyring,
            StorageBackendSetting::File => StorageBackend::File,
        }
    }
}

impl From<StorageBackend> for StorageBackendSetting {
    fn from(value: StorageBackend) -> Self {
        match value {
            StorageBackend::Keyring => StorageBackendSetting::Keyring,
            StorageBackend::File => StorageBackendSetting::File,
        }
    }
}

pub fn load_settings() -> Result<CredentialSettings> {
    let path = settings_path()?;
    if !path.exists() {
        return Ok(CredentialSettings::default());
    }
    serde_json::from_slice(
        &std::fs::read(&path).with_context(|| format!("Failed to read {}", path.display()))?,
    )
    .with_context(|| format!("Invalid {}", path.display()))
}

pub fn save_settings(settings: &CredentialSettings) -> Result<()> {
    let path = settings_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_vec_pretty(settings)?)?;
    Ok(())
}

pub fn credential_backend() -> Result<StorageBackend> {
    Ok(load_settings()?.credential_backend.into())
}

pub fn set_credential_backend(backend: StorageBackend) -> Result<()> {
    let mut settings = load_settings()?;
    settings.credential_backend = backend.into();
    save_settings(&settings)
}

fn settings_path() -> Result<PathBuf> {
    let home = timely_home().ok_or_else(|| anyhow::anyhow!("could not resolve timely home"))?;
    Ok(home.join(SETTINGS_FILE))
}

#[cfg(test)]
mod tests {
    use super::{CredentialSettings, StorageBackendSetting};

    #[test]
    fn defaults_to_keyring_backend() {
        let settings = CredentialSettings::default();
        assert_eq!(settings.credential_backend, StorageBackendSetting::Keyring);
    }
}
