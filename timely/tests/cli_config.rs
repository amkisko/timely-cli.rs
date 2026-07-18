use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::PathBuf;

fn timely() -> Command {
    Command::cargo_bin("timely").unwrap()
}

fn temp_home(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "timely-home-{}-{}-{}",
        label,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn config_path_uses_timely_home() {
    let home = temp_home("path");
    timely()
        .env("TIMELY_HOME", &home)
        .env_remove("TIMELY_CLIENT_ID")
        .args(["config", "path", "--plain"])
        .assert()
        .success()
        .stdout(predicate::str::contains(home.display().to_string()))
        .stdout(predicate::str::contains("config.env"));
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn config_set_get_round_trip_for_oauth_client_id() {
    let home = temp_home("set-get");
    timely()
        .env("TIMELY_HOME", &home)
        .env_remove("TIMELY_CLIENT_ID")
        .args(["config", "set", "oauth.client_id", "cli-client-id"])
        .assert()
        .success();

    let content = fs::read_to_string(home.join("config.env")).unwrap();
    assert!(content.contains("TIMELY_CLIENT_ID=cli-client-id"));

    timely()
        .env("TIMELY_HOME", &home)
        .env_remove("TIMELY_CLIENT_ID")
        .args(["config", "get", "oauth.client_id", "--plain"])
        .assert()
        .success()
        .stdout(predicate::str::contains("cli-client-id"));

    let _ = fs::remove_dir_all(&home);
}

#[test]
fn home_config_supplies_client_id_to_clap() {
    let home = temp_home("clap-load");
    fs::write(
        home.join("config.env"),
        "TIMELY_CLIENT_ID=from-home-config\n",
    )
    .unwrap();

    // With TIMELY_HOME loaded before parse, oauth uses the configured client id
    // (authorize URL) instead of failing clap for a missing --client-id flag.
    timely()
        .env("TIMELY_HOME", &home)
        .env_remove("TIMELY_CLIENT_ID")
        .env_remove("TIMELY_CLIENT_SECRET")
        .args(["auth", "oauth"])
        .write_stdin("")
        .assert()
        .failure()
        .stdout(predicate::str::contains("client_id=from-home-config"))
        .stderr(predicate::str::contains("--client-id").not());

    let _ = fs::remove_dir_all(&home);
}

#[test]
fn config_set_rejects_secrets() {
    let home = temp_home("reject-secret");
    timely()
        .env("TIMELY_HOME", &home)
        .args(["config", "set", "TIMELY_TOKEN", "nope"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("secrets are not stored"));
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn process_env_wins_over_config_file_for_list() {
    let home = temp_home("env-wins");
    fs::write(home.join("config.env"), "TIMELY_CLIENT_ID=from-file\n").unwrap();

    timely()
        .env("TIMELY_HOME", &home)
        .env("TIMELY_CLIENT_ID", "from-process")
        .args(["config", "get", "oauth.client_id", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("from-process"))
        .stdout(predicate::str::contains("\"source\":\"env\""));

    let _ = fs::remove_dir_all(&home);
}
