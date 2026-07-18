//! TTY detection and stderr feedback helpers.

use std::io::{self, IsTerminal, Write};

pub fn stdout_is_tty() -> bool {
    io::stdout().is_terminal()
}

pub fn stdin_is_tty() -> bool {
    io::stdin().is_terminal()
}

pub fn stderr_is_tty() -> bool {
    io::stderr().is_terminal()
}

pub fn color_enabled(no_color_flag: bool) -> bool {
    if no_color_flag {
        return false;
    }
    if std::env::var_os("TIMELY_NO_COLOR").is_some() || std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    if std::env::var("TERM").is_ok_and(|term| term == "dumb") {
        return false;
    }
    if std::env::var_os("FORCE_COLOR").is_some() {
        return true;
    }
    stderr_is_tty() || stdout_is_tty()
}

pub fn debug_enabled(debug_flag: bool) -> bool {
    debug_flag || std::env::var_os("DEBUG").is_some()
}

pub fn progress_message(quiet: bool, message: &str) {
    if quiet {
        return;
    }
    let _ = writeln!(io::stderr(), "{message}");
}

pub fn user_notice(quiet: bool, message: &str) {
    if quiet {
        return;
    }
    let _ = writeln!(io::stderr(), "{message}");
}

pub fn suggest_next_command(quiet: bool, message: &str) {
    if quiet {
        return;
    }
    let _ = writeln!(io::stderr(), "{message}");
}

pub fn print_concise_usage(command: Option<&str>, summary: &str, examples: &[&str]) {
    match command {
        Some(name) => {
            let _ = writeln!(io::stderr(), "timely {name} — {summary}\n");
        }
        None => {
            let _ = writeln!(io::stderr(), "timely — {summary}\n");
        }
    }
    let _ = writeln!(io::stderr(), "Examples:");
    for example in examples {
        let _ = writeln!(io::stderr(), "  {example}");
    }
    match command {
        Some(name) => {
            let _ = writeln!(
                io::stderr(),
                "\nRun `timely {name} --help` for all options."
            );
        }
        None => {
            let _ = writeln!(io::stderr(), "\nRun `timely --help` for all options.");
        }
    }
}

pub fn terminal_columns() -> usize {
    std::env::var("COLUMNS")
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|columns| *columns > 0)
        .unwrap_or(80)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_enabled_respects_no_color_flag() {
        assert!(!color_enabled(true));
    }
}
