use std::io::Write;
use std::process::Output;

use anyhow::{Context, Result};

pub(crate) fn write_stdin(mut child: std::process::Child, bytes: &[u8]) -> Result<Output> {
    {
        let stdin = child
            .stdin
            .as_mut()
            .context("subprocess stdin unavailable")?;
        stdin.write_all(bytes)?;
    }
    child
        .wait_with_output()
        .context("failed waiting for subprocess")
}
