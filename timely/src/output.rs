//! Output formatting for human, script, and JSON consumers.

use crate::cli_util::{stdout_is_tty, terminal_columns};
use crate::output_render::{format_human_plain, format_script_plain};
use serde_json::Value;
use std::io::Write as IoWrite;
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputMode {
    #[default]
    HumanPlain,
    ScriptPlain,
    JsonCompact,
    JsonPretty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Auto,
    Plain,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmitOptions {
    pub mode: OutputMode,
    pub use_color: bool,
}

pub fn resolve_output_mode(
    output: OutputFormat,
    json_flag: bool,
    plain_script_flag: bool,
    json_pretty_flag: bool,
    stdout_is_tty: bool,
) -> OutputMode {
    if json_flag {
        return OutputMode::JsonCompact;
    }
    if plain_script_flag {
        return OutputMode::ScriptPlain;
    }
    if json_pretty_flag {
        return OutputMode::JsonPretty;
    }
    match output {
        OutputFormat::Auto => {
            if stdout_is_tty {
                OutputMode::HumanPlain
            } else {
                OutputMode::JsonCompact
            }
        }
        OutputFormat::Plain => OutputMode::HumanPlain,
        OutputFormat::Json => OutputMode::JsonPretty,
    }
}

pub fn format_value(options: EmitOptions, value: &Value) -> Result<String, String> {
    match options.mode {
        OutputMode::HumanPlain => Ok(format_human_plain(
            value,
            terminal_columns(),
            options.use_color,
        )),
        OutputMode::ScriptPlain => Ok(format_script_plain(value)),
        OutputMode::JsonCompact => serde_json::to_string(value).map_err(|error| error.to_string()),
        OutputMode::JsonPretty => {
            serde_json::to_string_pretty(value).map_err(|error| error.to_string())
        }
    }
}

pub fn emit_value(options: EmitOptions, value: &Value) -> Result<(), String> {
    let formatted = format_value(options, value)?;
    emit_text(&formatted, options.mode == OutputMode::HumanPlain)
}

pub fn emit_text(text: &str, use_pager: bool) -> Result<(), String> {
    if use_pager
        && (stdout_is_tty() || crate::cli_util::stdin_is_tty())
        && text.lines().count() > terminal_rows()
    {
        pipe_to_pager(text)?;
    } else {
        print!("{text}");
        if !text.ends_with('\n') {
            println!();
        }
    }
    Ok(())
}

fn terminal_rows() -> usize {
    std::env::var("LINES")
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|rows| *rows > 0)
        .unwrap_or(24)
}

fn pipe_to_pager(text: &str) -> Result<(), String> {
    let pager = std::env::var("PAGER").unwrap_or_else(|_| "less".to_string());
    let pager_program = pager.split_whitespace().next().unwrap_or("less");
    let pager_args: Vec<&str> = pager.split_whitespace().skip(1).collect();

    let mut command = Command::new(pager_program);
    command.stdin(Stdio::piped()).stdout(Stdio::inherit());
    if pager_program == "less" && pager_args.is_empty() {
        command.arg("-FIRX");
    } else {
        command.args(pager_args);
    }

    let mut child = command
        .spawn()
        .map_err(|error| format!("start pager: {error}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(text.as_bytes())
            .map_err(|error| format!("write pager input: {error}"))?;
    }
    let status = child
        .wait()
        .map_err(|error| format!("wait for pager: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err("pager exited with an error".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_output_mode_prefers_flags() {
        assert_eq!(
            resolve_output_mode(OutputFormat::Plain, true, false, false, true),
            OutputMode::JsonCompact
        );
        assert_eq!(
            resolve_output_mode(OutputFormat::Json, false, true, false, true),
            OutputMode::ScriptPlain
        );
    }

    #[test]
    fn resolve_output_mode_auto_follows_tty() {
        assert_eq!(
            resolve_output_mode(OutputFormat::Auto, false, false, false, true),
            OutputMode::HumanPlain
        );
        assert_eq!(
            resolve_output_mode(OutputFormat::Auto, false, false, false, false),
            OutputMode::JsonCompact
        );
    }

    #[test]
    fn resolve_output_mode_explicit_json_overrides_auto() {
        assert_eq!(
            resolve_output_mode(OutputFormat::Json, false, false, false, true),
            OutputMode::JsonPretty
        );
    }
}
