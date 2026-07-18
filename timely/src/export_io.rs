//! Write export JSON to stdout or a file.

use std::fs;
use std::io::{self, Write};
use std::path::Path;

use anyhow::{Context, Result};
use serde_json::Value;

pub fn write_json_output(value: &Value, output: Option<&str>) -> Result<()> {
    let text = serde_json::to_string_pretty(value)?;
    match output {
        None | Some("-") => {
            let mut stdout = io::stdout().lock();
            writeln!(stdout, "{text}")?;
        }
        Some(path) => {
            fs::write(Path::new(path), format!("{text}\n"))
                .with_context(|| format!("failed to write export to {path}"))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn write_json_output_to_file() {
        let path =
            std::env::temp_dir().join(format!("timely-export-test-{}.json", std::process::id()));
        let value = json!({"ok": true});
        write_json_output(&value, Some(path.to_str().unwrap())).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        let _ = fs::remove_file(&path);
        assert!(text.contains("\"ok\": true"));
    }
}
