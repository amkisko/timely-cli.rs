use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread;
use std::time::Duration;

use super::*;
use serde_json::json;

use crate::cli::OauthCommand;

#[test]
fn parses_out_of_band_redirect_uri() {
    let oauth = oauth_command(DEFAULT_OAUTH_REDIRECT_URI);

    let target = redirect_target(&oauth).unwrap();

    match target {
        RedirectTarget::OutOfBand { redirect_uri } => {
            assert_eq!(redirect_uri, DEFAULT_OAUTH_REDIRECT_URI);
        }
        RedirectTarget::Loopback(_) => panic!("expected out-of-band redirect target"),
    }
}

#[test]
fn extracts_listener_host_port_and_path_from_loopback_redirect_uri() {
    let oauth = oauth_command("http://127.0.0.1:9123/oauth/callback");

    let target = redirect_target(&oauth).unwrap();

    let RedirectTarget::Loopback(listener) = target else {
        panic!("expected loopback redirect target");
    };

    assert_eq!(listener.host, "127.0.0.1");
    assert_eq!(listener.port, 9123);
    assert_eq!(listener.path, "/oauth/callback");
    assert_eq!(
        listener.redirect_uri,
        "http://127.0.0.1:9123/oauth/callback"
    );
}

#[test]
fn rejects_redirect_uri_without_explicit_port() {
    let oauth = oauth_command("http://127.0.0.1/callback");

    let error = redirect_target(&oauth).unwrap_err().to_string();

    assert!(error.contains("explicit port"));
}

#[test]
fn rejects_non_loopback_redirect_host() {
    let oauth = oauth_command("http://example.com:9123/oauth/callback");

    let error = redirect_target(&oauth).unwrap_err().to_string();

    assert!(error.contains("loopback"));
}

#[test]
fn accepts_localhost_loopback_redirect_uri() {
    let oauth = oauth_command("http://localhost:9123/oauth/callback");

    let target = redirect_target(&oauth).unwrap();

    let RedirectTarget::Loopback(listener) = target else {
        panic!("expected loopback redirect target");
    };
    assert_eq!(listener.host, "localhost");
}

#[test]
fn extracts_code_from_matching_request_path_and_state() {
    let code = oauth_code_from_request(
        "/oauth/callback?state=expected-state&code=oauth-code",
        "/oauth/callback",
        "expected-state",
    )
    .unwrap();

    assert_eq!(code, "oauth-code");
}

#[test]
fn rejects_callback_request_with_wrong_path() {
    let error = oauth_code_from_request(
        "/wrong/path?state=expected-state&code=oauth-code",
        "/oauth/callback",
        "expected-state",
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("path mismatch"));
}

#[test]
fn extracts_code_from_pasted_callback_url() {
    let code =
        oauth_code_from_prompt("https://example.com/callback?state=expected-state&code=oauth-code")
            .unwrap();

    assert_eq!(code, "oauth-code");
}

#[test]
fn trims_pasted_out_of_band_code() {
    let code = oauth_code_from_prompt("  oauth-code\n").unwrap();

    assert_eq!(code, "oauth-code");
}

#[test]
fn parses_token_response() {
    let value = json!({"access_token":"a","token_type":"Bearer","scope":"manage","created_at":1});
    let credential = timely_lib::oauth::credential_from_token_response(value).unwrap();
    assert_eq!(credential.access_token, "a");
    assert_eq!(credential.scope.as_deref(), Some("manage"));
    assert_eq!(credential.account_id, None);
}

fn is_pkce_url_safe(value: &str) -> bool {
    value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
}

#[test]
fn pkce_verifier_meets_length_and_charset_requirements() {
    let verifier = pkce_verifier().unwrap();
    assert!(verifier.len() >= 43);
    assert!(is_pkce_url_safe(&verifier));
}

#[test]
fn oauth_state_is_independent_random_value() {
    let verifier = pkce_verifier().unwrap();
    let state = oauth_state().unwrap();
    assert!(state.len() >= 43);
    assert!(is_pkce_url_safe(&state));
    assert_ne!(verifier, state);
}

#[test]
fn pkce_verifier_values_differ_across_calls() {
    let first = pkce_verifier().unwrap();
    let second = pkce_verifier().unwrap();
    assert_ne!(first, second);
}

#[test]
fn loopback_ignores_noise_then_accepts_callback() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let expected_state = "expected-state".to_string();
    let join_handle = thread::spawn(move || {
        wait_for_oauth_code_until(
            "127.0.0.1",
            port,
            "/oauth/callback",
            &expected_state,
            Instant::now() + Duration::from_secs(5),
        )
    });

    thread::sleep(Duration::from_millis(50));
    send_http_get(port, "/favicon.ico");
    send_http_get(port, "/oauth/callback?state=expected-state&code=oauth-code");

    let code = join_handle.join().unwrap().unwrap();
    assert_eq!(code, "oauth-code");
}

fn send_http_get(port: u16, path: &str) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream
        .write_all(
            format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
                .as_bytes(),
        )
        .unwrap();
    let mut buffer = Vec::new();
    let _ = stream.read_to_end(&mut buffer);
}

fn oauth_command(redirect_uri: &str) -> OauthCommand {
    OauthCommand {
        client_id: "client-id".to_string(),
        client_secret: None,
        client_secret_file: None,
        scope: "manage".to_string(),
        redirect_uri: redirect_uri.to_string(),
        open: false,
        no_open: false,
    }
}
