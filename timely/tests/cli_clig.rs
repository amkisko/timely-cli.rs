use assert_cmd::Command;
use predicates::prelude::*;

fn timely() -> Command {
    Command::cargo_bin("timely").unwrap()
}

#[test]
fn auth_token_rejects_both_token_sources() {
    timely()
        .arg("auth")
        .arg("token")
        .arg("--token")
        .arg("secret")
        .arg("--token-file")
        .arg("/tmp/unused")
        .assert()
        .failure()
        .code(2);
}

#[test]
fn auth_token_rejects_empty_token_file() {
    let path = std::env::temp_dir().join(format!("timely-empty-token-{}", std::process::id()));
    std::fs::write(&path, "").unwrap();
    timely()
        .env_remove("TIMELY_TOKEN")
        .arg("auth")
        .arg("token")
        .arg("--token-file")
        .arg(&path)
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("empty"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn auth_token_requires_a_source() {
    timely()
        .env_remove("TIMELY_TOKEN")
        .arg("auth")
        .arg("token")
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("provide --token"));
}

#[test]
fn destructive_delete_requires_yes_when_non_interactive() {
    timely()
        .arg("api")
        .arg("time-entries")
        .arg("delete")
        .arg("789")
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("pass --yes"));
}

#[test]
fn destructive_delete_dry_run_skips_api() {
    timely()
        .arg("--dry-run")
        .arg("api")
        .arg("time-entries")
        .arg("delete")
        .arg("789")
        .assert()
        .success()
        .stdout(predicate::str::contains("dry_run"))
        .stdout(predicate::str::contains("delete time entry 789"))
        .stderr(predicate::str::contains("Dry run"));
}

#[test]
fn json_flag_forces_compact_json() {
    timely()
        .arg("spec")
        .arg("summary")
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"operations\":"))
        .stdout(predicate::str::contains("\n  ").not());
}

#[test]
fn help_lists_command_about_text() {
    timely()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Manage authentication"))
        .stdout(predicate::str::contains("HTTP timeout"));
}

#[test]
fn close_subcommand_typo_suggests_correction() {
    timely()
        .arg("authe")
        .assert()
        .failure()
        .stderr(predicate::str::contains("auth"));
}
