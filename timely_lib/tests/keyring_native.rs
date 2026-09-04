//! Verifies keyring platform features are enabled (not the in-memory mock store).

use keyring::Entry;
use timely_lib::{keyring_backend_label, KEYRING_SERVICE};

#[test]
fn reports_platform_keyring_backend() {
    let label = keyring_backend_label();
    #[cfg(target_os = "macos")]
    assert_eq!(label, "macOS Keychain");
    #[cfg(target_os = "windows")]
    assert_eq!(label, "Windows Credential Manager");
    #[cfg(target_os = "linux")]
    assert_eq!(label, "Secret Service (libsecret)");
}

#[test]
fn os_keyring_roundtrip() {
    let user = format!("native-probe-{}", std::process::id());
    let entry = Entry::new(KEYRING_SERVICE, &user).expect("open keyring entry");
    if let Err(keyring::Error::PlatformFailure(_)) = entry.set_password("roundtrip") {
        eprintln!("skip: OS keychain unavailable in this environment");
        return;
    }
    assert_eq!(
        entry.get_password().expect("read password from OS keyring"),
        "roundtrip"
    );
    let _ = entry.delete_credential();
}
