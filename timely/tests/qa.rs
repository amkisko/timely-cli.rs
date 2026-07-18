use std::fs;
use std::path::{Path, PathBuf};

const MAX_LINES: usize = 299;
const MAX_BYTES: u64 = 32 * 1024 - 1;
const MAX_LINE_CHARS: usize = 120;

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

#[test]
fn repository_files_stay_small_and_wrapped() {
    let root = repository_root();
    let mut failures = Vec::new();

    for path in first_party_files(&root) {
        if is_excluded_policy_file(&path, &root) || is_vendored_openapi_json(&path, &root) {
            continue;
        }

        let metadata = fs::metadata(&path).unwrap();
        if metadata.len() > MAX_BYTES {
            failures.push(format!("{} is {} bytes", rel(&root, &path), metadata.len()));
        }

        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let line_count = text.lines().count();
        if line_count > MAX_LINES {
            failures.push(format!("{} has {line_count} lines", rel(&root, &path)));
        }
        for (index, line) in text.lines().enumerate() {
            let width = line.chars().count();
            if width > MAX_LINE_CHARS {
                failures.push(format!(
                    "{}:{} has {width} chars",
                    rel(&root, &path),
                    index + 1
                ));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "QA limits failed:\n{}",
        failures.join("\n")
    );
}

#[test]
fn repository_does_not_use_banned_runtimes() {
    let root = repository_root();
    let banned_exts = ["js", "mjs", "cjs", "py", "go"];
    let mut failures = Vec::new();

    for path in first_party_files(&root) {
        if is_excluded_policy_file(&path, &root) {
            continue;
        }
        if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| banned_exts.contains(&ext))
        {
            failures.push(rel(&root, &path));
        }
    }

    assert!(
        failures.is_empty(),
        "banned runtime files found:\n{}",
        failures.join("\n")
    );
}

fn first_party_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    visit(root, &mut files);
    files.sort();
    files
}

fn visit(path: &Path, files: &mut Vec<PathBuf>) {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    if matches!(
        name,
        ".git" | "target" | "tmp" | ".github" | "Cargo.lock" | ".pray"
    ) {
        return;
    }
    if path.is_file() {
        files.push(path.to_path_buf());
        return;
    }
    for entry in fs::read_dir(path).unwrap() {
        visit(&entry.unwrap().path(), files);
    }
}

fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root).unwrap().display().to_string()
}

/// Pretty-printed Timely OpenAPI from `scripts/update-openapi.rb` is intentionally large.
fn is_vendored_openapi_json(path: &Path, root: &Path) -> bool {
    path.strip_prefix(root)
        .ok()
        .and_then(|p| p.to_str())
        .is_some_and(|p| p == "openapi/openapi.json" || p == "tmp/openapi/openapi.json")
}

fn is_excluded_policy_file(path: &Path, root: &Path) -> bool {
    path.strip_prefix(root)
        .ok()
        .and_then(|p| p.to_str())
        .is_some_and(|p| {
            matches!(
                p,
                "CODE_OF_CONDUCT.md" | "SECURITY.md" | "AGENTS.md" | "Prayfile.lock"
            ) || p.starts_with(".agents/")
        })
}
