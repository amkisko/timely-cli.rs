use assert_cmd::Command;
use predicates::prelude::*;

fn timely() -> Command {
    Command::cargo_bin("timely").unwrap()
}

#[test]
fn version_flag_prints_version() {
    timely()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("timely 0.1.0"));
}

#[test]
fn version_subcommand_prints_version() {
    timely()
        .arg("version")
        .assert()
        .success()
        .stdout(predicate::str::contains("timely 0.1.0"));
}

#[test]
fn help_flag_shows_usage() {
    timely()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Query Timely"));
}

#[test]
fn top_level_help_documents_exit_codes() {
    timely()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Exit codes"));
}

#[test]
fn spec_summary_works_without_credentials() {
    timely()
        .arg("spec")
        .arg("summary")
        .assert()
        .success()
        .stdout(predicate::str::contains("operations"));
}

#[test]
fn spec_summary_json_output() {
    timely()
        .arg("spec")
        .arg("summary")
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"operations\""));
}

#[test]
fn global_json_flag_after_subcommand() {
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
fn missing_token_reports_auth_exit_code() {
    let profile = format!("timely-cli-test-{}", std::process::id());

    timely()
        .env_remove("TIMELY_TOKEN")
        .env_remove("TIMELY_ACCOUNT_ID")
        .arg("--profile")
        .arg(&profile)
        .arg("api")
        .arg("me")
        .assert()
        .failure()
        .code(3)
        .stderr(predicate::str::contains("account_id required"));
}

#[test]
fn quiet_still_prints_errors() {
    let profile = format!("timely-cli-quiet-{}", std::process::id());

    timely()
        .env_remove("TIMELY_TOKEN")
        .arg("--profile")
        .arg(&profile)
        .arg("api")
        .arg("me")
        .arg("--quiet")
        .assert()
        .failure()
        .code(3)
        .stderr(predicate::str::contains("account_id required"));
}

#[test]
fn completions_bash_generates_script() {
    timely()
        .arg("completions")
        .arg("bash")
        .assert()
        .success()
        .stdout(predicate::str::contains("_timely"));
}

#[test]
fn man_command_prints_roff() {
    timely()
        .arg("man")
        .assert()
        .success()
        .stdout(predicate::str::contains(".TH"));
}

#[test]
fn batch_local_ops_without_credentials() {
    timely()
        .arg("batch")
        .write_stdin(
            r#"[{"id":"spec","args":["spec","summary"]},{"id":"auth","args":["auth","status"]}]"#,
        )
        .assert()
        .success()
        .stdout(predicate::str::contains("\"succeeded\":2"))
        .stdout(predicate::str::contains("operations"))
        .stdout(predicate::str::contains("token_configured"));
}

#[test]
fn batch_rejects_nested_batch() {
    timely()
        .arg("batch")
        .write_stdin(r#"[{"args":["batch"]}]"#)
        .assert()
        .failure()
        .code(2)
        .stdout(predicate::str::contains("nested batch is not supported"));
}

#[test]
fn batch_rejects_auth_token() {
    timely()
        .arg("batch")
        .write_stdin(r#"[{"args":["auth","token","--token","secret"]}]"#)
        .assert()
        .failure()
        .code(2)
        .stdout(predicate::str::contains("cannot run inside batch"));
}

#[test]
fn batch_unknown_operation_returns_usage_exit() {
    timely()
        .arg("batch")
        .write_stdin(r#"[{"args":["not-a-real-command"]}]"#)
        .assert()
        .failure()
        .code(2)
        .stdout(
            predicate::str::contains("unrecognized subcommand")
                .or(predicate::str::contains("error:")),
        );
}

#[test]
fn batch_help_documents_plan_format() {
    timely()
        .arg("batch")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("operations"))
        .stdout(predicate::str::contains("--file"));
}

#[test]
fn batch_empty_stdin_returns_usage_exit() {
    timely()
        .arg("batch")
        .write_stdin("")
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("batch stdin is empty"));
}

#[test]
fn batch_invalid_json_returns_usage_exit() {
    timely()
        .arg("batch")
        .write_stdin("not json")
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("batch input must be a JSON array"));
}

#[test]
fn batch_partial_failure_prints_stderr_summary() {
    timely()
        .arg("batch")
        .write_stdin(r#"[{"args":["spec","summary"]},{"args":["batch"]}]"#)
        .assert()
        .failure()
        .stderr(predicate::str::contains("1 of 2 operations failed"));
}

#[test]
fn batch_json_pretty_formats_report() {
    timely()
        .arg("batch")
        .arg("--json-pretty")
        .write_stdin(r#"[{"args":["spec","summary"]}]"#)
        .assert()
        .success()
        .stdout(predicate::str::contains("\n  \"succeeded\": 1"));
}

#[test]
fn batch_fail_fast_stops_after_first_failure() {
    timely()
        .arg("batch")
        .arg("--fail-fast")
        .write_stdin(
            r#"[{"args":["batch"]},{"args":["spec","summary"]},{"args":["auth","status"]}]"#,
        )
        .assert()
        .failure()
        .code(2)
        .stdout(predicate::str::contains("\"operations\":1"))
        .stdout(predicate::str::contains("nested batch is not supported"))
        .stdout(predicate::str::contains("token_configured").not())
        .stderr(predicate::str::contains("Stopped after first failure"));
}

#[test]
fn batch_without_fail_fast_runs_remaining_operations() {
    timely()
        .arg("batch")
        .write_stdin(r#"[{"args":["batch"]},{"args":["auth","status"]}]"#)
        .assert()
        .failure()
        .code(2)
        .stdout(predicate::str::contains("\"operations\":2"))
        .stdout(predicate::str::contains("token_configured"));
}

#[test]
fn no_color_plain_output_has_no_ansi() {
    timely()
        .arg("spec")
        .arg("operations")
        .arg("-o")
        .arg("plain")
        .arg("--no-color")
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[").not());
}

#[test]
fn unknown_operation_reports_usage_exit() {
    timely()
        .arg("call")
        .arg("not_a_real_operation")
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("unknown operationId"));
}
