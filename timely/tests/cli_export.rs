use assert_cmd::Command;
use predicates::prelude::*;

fn timely() -> Command {
    Command::cargo_bin("timely").unwrap()
}

#[test]
fn auth_export_help_documents_file_flag() {
    timely()
        .arg("auth")
        .arg("export")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--file"));
}

#[cfg(feature = "memory")]
#[test]
fn memory_export_help_documents_filters() {
    timely()
        .arg("memory")
        .arg("export")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--since"))
        .stdout(predicate::str::contains("--file"));
}

#[cfg(feature = "memory")]
#[test]
fn memory_export_missing_db_returns_clear_error() {
    let missing = std::env::temp_dir().join(format!(
        "timely-missing-memory-{}.sqlite",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&missing);
    timely()
        .arg("memory")
        .arg("export")
        .arg("--db-path")
        .arg(&missing)
        .assert()
        .failure()
        .stderr(predicate::str::contains("Memory database not found"));
}

#[cfg(not(feature = "memory"))]
#[test]
fn memory_command_is_unavailable_without_memory_feature() {
    timely().arg("memory").assert().failure();
    timely()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("local Memory reader").not());
}
