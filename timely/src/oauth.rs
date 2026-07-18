use std::collections::BTreeMap;
use std::io::{self, Write};
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow, bail};
use base64::Engine;
use sha2::{Digest, Sha256};
use tiny_http::{Response, Server, StatusCode};
use url::Url;

use timely_lib::Api;
use timely_lib::StoredCredential;
use timely_lib::oauth::{OAuthExchange, exchange_authorization_code};
use timely_lib::util::join_url;

use crate::cli::{DEFAULT_OAUTH_REDIRECT_URI, OauthCommand};

const PKCE_VERIFIER_BYTES: usize = 32;
const OAUTH_STATE_BYTES: usize = 32;
const OAUTH_CALLBACK_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const OAUTH_RECV_POLL: Duration = Duration::from_secs(1);

#[derive(Debug)]
struct RedirectListener {
    host: String,
    port: u16,
    path: String,
    redirect_uri: String,
}

#[derive(Debug)]
enum RedirectTarget {
    Loopback(RedirectListener),
    OutOfBand { redirect_uri: String },
}

pub async fn run_oauth_flow(
    api: &Api,
    oauth: &OauthCommand,
    client_secret: Option<String>,
) -> Result<StoredCredential> {
    let redirect_target = redirect_target(oauth)?;
    let verifier = pkce_verifier()?;
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(Sha256::digest(verifier.as_bytes()));
    let state = oauth_state()?;
    let authorize = authorize_url(
        api,
        oauth,
        redirect_target.redirect_uri(),
        &challenge,
        &state,
    )?;

    println!("Open this URL in your browser to authorize Timely:");
    println!("{authorize}");
    if oauth.should_open_browser() {
        let _ = open::that(authorize.as_str());
    }

    let code = match &redirect_target {
        RedirectTarget::Loopback(listener) => {
            wait_for_oauth_code(&listener.host, listener.port, &listener.path, &state)?
        }
        RedirectTarget::OutOfBand { .. } => prompt_for_oauth_code()?,
    };
    exchange_authorization_code(
        api,
        &OAuthExchange {
            client_id: oauth.client_id.clone(),
            client_secret,
            redirect_uri: redirect_target.redirect_uri().to_string(),
            code,
            verifier,
        },
    )
    .await
}

fn redirect_target(oauth: &OauthCommand) -> Result<RedirectTarget> {
    if oauth.redirect_uri == DEFAULT_OAUTH_REDIRECT_URI {
        return Ok(RedirectTarget::OutOfBand {
            redirect_uri: oauth.redirect_uri.clone(),
        });
    }

    let parsed_redirect_uri = Url::parse(&oauth.redirect_uri).map_err(|error| {
        anyhow!(
            "invalid OAuth redirect URI `{}`: {error}",
            oauth.redirect_uri
        )
    })?;
    if parsed_redirect_uri.scheme() != "http" {
        bail!("OAuth redirect URI must use http scheme");
    }

    let host = parsed_redirect_uri
        .host_str()
        .ok_or_else(|| anyhow!("OAuth redirect URI must include a host"))?
        .to_string();
    if !is_loopback_host(&host) {
        bail!("OAuth redirect URI host must be a loopback address");
    }
    let port = parsed_redirect_uri
        .port()
        .ok_or_else(|| anyhow!("OAuth redirect URI must include an explicit port"))?;

    Ok(RedirectTarget::Loopback(RedirectListener {
        host,
        port,
        path: parsed_redirect_uri.path().to_string(),
        redirect_uri: oauth.redirect_uri.clone(),
    }))
}

impl RedirectTarget {
    fn redirect_uri(&self) -> &str {
        match self {
            Self::Loopback(listener) => listener.redirect_uri.as_str(),
            Self::OutOfBand { redirect_uri } => redirect_uri.as_str(),
        }
    }
}

fn authorize_url(
    api: &Api,
    oauth: &OauthCommand,
    redirect_uri: &str,
    challenge: &str,
    state: &str,
) -> Result<Url> {
    let mut authorize = Url::parse(&join_url(&api.base_url, "/1.1/oauth/authorize")?)?;
    authorize
        .query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", &oauth.client_id)
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("scope", &oauth.scope)
        .append_pair("state", state)
        .append_pair("code_challenge", challenge)
        .append_pair("code_challenge_method", "S256");
    Ok(authorize)
}

fn wait_for_oauth_code(
    host: &str,
    port: u16,
    expected_path: &str,
    expected_state: &str,
) -> Result<String> {
    wait_for_oauth_code_until(
        host,
        port,
        expected_path,
        expected_state,
        Instant::now() + OAUTH_CALLBACK_TIMEOUT,
    )
}

fn wait_for_oauth_code_until(
    host: &str,
    port: u16,
    expected_path: &str,
    expected_state: &str,
    deadline: Instant,
) -> Result<String> {
    let bind_address = bind_address(host, port);
    let server = Server::http(bind_address.as_str()).map_err(|error| anyhow!("{error}"))?;
    loop {
        if Instant::now() >= deadline {
            bail!("OAuth callback timed out waiting for browser redirect");
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        let poll = remaining.min(OAUTH_RECV_POLL);
        let Some(request) = server
            .recv_timeout(poll)
            .map_err(|error| anyhow!("{error}"))?
        else {
            continue;
        };

        match oauth_code_from_request(request.url(), expected_path, expected_state) {
            Ok(code) => {
                request.respond(Response::from_string(
                    "Timely CLI is authenticated. You can close this tab.",
                ))?;
                return Ok(code);
            }
            Err(_) => {
                let _ = request.respond(
                    Response::from_string("Waiting for Timely OAuth callback.")
                        .with_status_code(StatusCode(404)),
                );
            }
        }
    }
}

fn is_loopback_host(host: &str) -> bool {
    matches!(host, "127.0.0.1" | "localhost" | "::1")
}

fn prompt_for_oauth_code() -> Result<String> {
    if !crate::cli_util::stdin_is_tty() {
        bail!(
            "refusing to prompt for an OAuth code on a non-interactive stdin; \
             use a loopback --redirect-uri instead"
        );
    }
    print!("Paste the Timely authorization code: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    oauth_code_from_prompt(&input)
}

fn bind_address(host: &str, port: u16) -> String {
    if host.contains(':') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}

fn oauth_code_from_request(
    request_url: &str,
    expected_path: &str,
    expected_state: &str,
) -> Result<String> {
    let url = Url::parse(&format!("http://listener{request_url}"))?;
    if url.path() != expected_path {
        bail!(
            "OAuth redirect path mismatch: expected `{expected_path}` but received `{}`",
            url.path()
        );
    }

    let pairs = url.query_pairs().into_owned().collect::<BTreeMap<_, _>>();
    let state = pairs
        .get("state")
        .ok_or_else(|| anyhow!("OAuth redirect missing state"))?;
    if state != expected_state {
        bail!("OAuth state mismatch");
    }

    pairs
        .get("code")
        .cloned()
        .ok_or_else(|| anyhow!("OAuth redirect missing code"))
}

fn oauth_code_from_prompt(input: &str) -> Result<String> {
    let trimmed_input = input.trim();
    if trimmed_input.is_empty() {
        bail!("OAuth code cannot be empty");
    }

    if let Ok(url) = Url::parse(trimmed_input)
        && let Some(code) = url
            .query_pairs()
            .find_map(|(key, value)| (key == "code").then(|| value.into_owned()))
    {
        return Ok(code);
    }

    Ok(trimmed_input.to_string())
}

fn pkce_verifier() -> Result<String> {
    random_url_safe_token(PKCE_VERIFIER_BYTES)
}

fn oauth_state() -> Result<String> {
    random_url_safe_token(OAUTH_STATE_BYTES)
}

fn random_url_safe_token(byte_count: usize) -> Result<String> {
    let mut bytes = vec![0u8; byte_count];
    getrandom::fill(&mut bytes)
        .map_err(|error| anyhow!("failed to generate OAuth random token: {error}"))?;
    Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes))
}

#[path = "oauth_tests.rs"]
#[cfg(test)]
mod tests;
