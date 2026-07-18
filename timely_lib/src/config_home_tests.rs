use super::*;
use std::sync::{Mutex, MutexGuard};

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn env_test_lock() -> MutexGuard<'static, ()> {
    ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[allow(unsafe_code)]
fn set_env(key: &str, value: &str) {
    unsafe { std::env::set_var(key, value) };
}

#[allow(unsafe_code)]
fn remove_env(key: &str) {
    unsafe { std::env::remove_var(key) };
}

#[test]
fn set_and_get_config_entry_round_trip() {
    let _guard = env_test_lock();
    let home = std::env::temp_dir().join(format!("timely-config-cli-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    let original_home = std::env::var("TIMELY_HOME").ok();
    set_env("TIMELY_HOME", home.to_string_lossy().as_ref());
    remove_env("TIMELY_CLIENT_ID");

    set_config_entry("oauth.client_id", "from-file").unwrap();
    let entry = get_config_entry("oauth.client_id").unwrap();
    assert_eq!(entry.value.as_deref(), Some("from-file"));
    assert_eq!(entry.source, Some(ConfigSource::File));
    unset_config_entry("oauth.client_id").unwrap();
    let entry = get_config_entry("oauth.client_id").unwrap();
    assert_eq!(entry.value, None);

    remove_env("TIMELY_HOME");
    if let Some(value) = original_home {
        set_env("TIMELY_HOME", &value);
    }
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn load_config_directory_applies_local_overrides() {
    let _guard = env_test_lock();
    let home = std::env::temp_dir().join(format!("timely-config-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    fs::create_dir_all(&home).unwrap();
    fs::write(home.join(CONFIG_FILE), "TIMELY_CLIENT_ID=from-config\n").unwrap();
    fs::write(
        home.join(LOCAL_CONFIG_FILE),
        "TIMELY_CLIENT_ID=from-local\nTIMELY_OUTPUT=json\n",
    )
    .unwrap();

    let original_client = std::env::var("TIMELY_CLIENT_ID").ok();
    let original_output = std::env::var("TIMELY_OUTPUT").ok();
    remove_env("TIMELY_CLIENT_ID");
    remove_env("TIMELY_OUTPUT");

    load_config_directory(&home).unwrap();

    assert_eq!(std::env::var("TIMELY_CLIENT_ID").unwrap(), "from-local");
    assert_eq!(std::env::var("TIMELY_OUTPUT").unwrap(), "json");

    remove_env("TIMELY_CLIENT_ID");
    remove_env("TIMELY_OUTPUT");
    if let Some(value) = original_client {
        set_env("TIMELY_CLIENT_ID", &value);
    }
    if let Some(value) = original_output {
        set_env("TIMELY_OUTPUT", &value);
    }
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn load_config_directory_does_not_override_process_env() {
    let _guard = env_test_lock();
    let home =
        std::env::temp_dir().join(format!("timely-config-precedence-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    fs::create_dir_all(&home).unwrap();
    fs::write(home.join(CONFIG_FILE), "TIMELY_CLIENT_ID=from-file\n").unwrap();

    let original_client = std::env::var("TIMELY_CLIENT_ID").ok();
    set_env("TIMELY_CLIENT_ID", "from-env");

    load_config_directory(&home).unwrap();

    assert_eq!(std::env::var("TIMELY_CLIENT_ID").unwrap(), "from-env");

    remove_env("TIMELY_CLIENT_ID");
    if let Some(value) = original_client {
        set_env("TIMELY_CLIENT_ID", &value);
    }
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn timely_home_honors_timely_home_env() {
    let _guard = env_test_lock();
    let original = std::env::var("TIMELY_HOME").ok();
    set_env("TIMELY_HOME", "/tmp/custom-timely-home");
    assert_eq!(
        timely_home().map(|path| path.display().to_string()),
        Some("/tmp/custom-timely-home".to_string())
    );
    remove_env("TIMELY_HOME");
    if let Some(value) = original {
        set_env("TIMELY_HOME", &value);
    }
}
