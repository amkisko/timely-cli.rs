use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result};
use keyring::Entry;
use reqwest::Client;

use crate::auth::StoredCredential;

const SERVICE: &str = "timely-cli";

#[derive(Clone, Debug)]
pub struct Api {
    pub profile: String,
    pub base_url: String,
    pub client: Client,
    current_user_ids: Arc<Mutex<BTreeMap<i64, i64>>>,
}

impl Api {
    pub fn new(profile: String, base_url: String, timeout_secs: Option<u64>) -> Self {
        let mut builder = Client::builder();
        if let Some(timeout_secs) = timeout_secs {
            builder = builder.timeout(Duration::from_secs(timeout_secs));
        }
        Self {
            profile,
            base_url: base_url.trim_end_matches('/').to_string(),
            client: builder.build().expect("failed to build Timely HTTP client"),
            current_user_ids: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    pub(crate) fn cached_current_user_id(&self, account_id: i64) -> Option<i64> {
        self.current_user_ids
            .lock()
            .ok()
            .and_then(|guard| guard.get(&account_id).copied())
    }

    pub(crate) fn store_cached_current_user_id(&self, account_id: i64, user_id: i64) {
        if let Ok(mut guard) = self.current_user_ids.lock() {
            guard.insert(account_id, user_id);
        }
    }

    pub fn cached_current_user_ids(&self) -> BTreeMap<i64, i64> {
        self.current_user_ids
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    pub fn store_credential(&self, credential: StoredCredential) -> Result<()> {
        let value = serde_json::to_string(&credential)?;
        self.keyring_entry()?
            .set_password(&value)
            .context("failed to store credential in OS keyring")
    }

    pub fn load_credential(&self) -> Result<Option<StoredCredential>> {
        match self.keyring_entry()?.get_password() {
            Ok(value) => Ok(Some(serde_json::from_str(&value)?)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(err) => Err(err).context("failed to read credential from OS keyring"),
        }
    }

    pub fn delete_credential(&self) -> Result<()> {
        match self.keyring_entry()?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(err) => Err(err).context("failed to delete credential from OS keyring"),
        }
    }

    pub(crate) fn keyring_entry(&self) -> Result<Entry> {
        Entry::new(SERVICE, &self.profile).context("failed to open OS keyring entry")
    }
}

// OpenAPI operation methods (build.rs → OUT_DIR/api_methods.rs). `call` / MCP still use `send`.
include!(concat!(env!("OUT_DIR"), "/api_methods.rs"));

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;
    use std::time::{Duration, Instant};

    use super::Api;
    use crate::oauth::{OAuthExchange, exchange_authorization_code};

    #[tokio::test]
    async fn current_user_id_uses_process_cache() {
        let api = Api::new(
            "cache-test".to_string(),
            "https://api.timelyapp.com".to_string(),
            Some(1),
        );
        api.store_cached_current_user_id(9, 42);
        assert_eq!(api.current_user_id(Some(9)).await.unwrap(), 42);
    }

    #[tokio::test]
    async fn http_client_respects_configured_timeout() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind stalled server");
        let address = listener.local_addr().expect("local address");
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut buffer = [0_u8; 1024];
            let _ = stream.read(&mut buffer);
            thread::sleep(Duration::from_secs(5));
            let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}");
        });

        let api = Api::new(
            "timeout-test".to_string(),
            format!("http://{address}"),
            Some(1),
        );
        let started = Instant::now();
        let error = exchange_authorization_code(
            &api,
            &OAuthExchange {
                client_id: "client".to_string(),
                client_secret: None,
                redirect_uri: "http://127.0.0.1/callback".to_string(),
                code: "code".to_string(),
                verifier: "verifier".to_string(),
            },
        )
        .await
        .expect_err("stalled token endpoint should time out");

        assert!(
            started.elapsed() < Duration::from_secs(3),
            "request should fail within the configured timeout, took {:?}",
            started.elapsed()
        );
        let message = format!("{error:#}").to_lowercase();
        assert!(
            message.contains("timed out")
                || message.contains("timeout")
                || message.contains("deadline")
                || message.contains("error sending request"),
            "unexpected timeout error: {error:#}"
        );
    }
}
