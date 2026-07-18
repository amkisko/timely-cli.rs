//! Human-oriented CLI error rendering.

use std::io::{self, Write};

use crate::exit::{AppExit, exit_for_message, exit_for_timely_error};
use timely_lib::TimelyError;

const ISSUES_URL: &str = "https://github.com/amkisko/timely-cli.rs/issues/new";

#[derive(Clone)]
pub struct ErrorContext {
    pub quiet: bool,
    pub verbose: bool,
    pub debug: bool,
}

pub fn print_error(message: &str, context: &ErrorContext) -> AppExit {
    let _ = writeln!(io::stderr(), "Error: {message}");
    if !context.quiet {
        print_message_hints(message);
        if context.verbose || context.debug {
            let _ = writeln!(io::stderr(), "Details: {message}");
        }
    }
    exit_for_message(message)
}

pub fn print_timely_error(error: &TimelyError, context: &ErrorContext) -> AppExit {
    let _ = writeln!(io::stderr(), "Error: {error}");
    if !context.quiet {
        print_timely_hints(error);
        if context.verbose || context.debug {
            let _ = writeln!(io::stderr(), "Details: {error}");
        }
        if matches!(error, TimelyError::Other(_)) {
            let _ = writeln!(
                io::stderr(),
                "\nReport a bug: {ISSUES_URL} (timely {})",
                env!("CARGO_PKG_VERSION")
            );
        }
    }
    exit_for_timely_error(error)
}

fn print_timely_hints(error: &TimelyError) {
    match error {
        TimelyError::Auth(_) => {
            let _ = writeln!(
                io::stderr(),
                "Hint: run `timely auth status` or `timely auth token --token ...`."
            );
            let _ = writeln!(
                io::stderr(),
                "      OAuth: `timely auth oauth --client-id ...`"
            );
        }
        TimelyError::Api(message) => {
            if message.contains("401") {
                let _ = writeln!(
                    io::stderr(),
                    "Hint: authentication failed — run `timely auth status`."
                );
            } else if message.contains("404") {
                let _ = writeln!(
                    io::stderr(),
                    "Hint: resource not found. Check account and resource IDs."
                );
            }
        }
        TimelyError::Usage(message) if message.contains("operationId") => {
            let _ = writeln!(
                io::stderr(),
                "Hint: run `timely spec operations` to list operation IDs."
            );
        }
        _ => {}
    }
}

fn print_message_hints(message: &str) {
    let lower = message.to_lowercase();
    if lower.contains("no token configured") || lower.contains("account_id required") {
        let _ = writeln!(
            io::stderr(),
            "Hint: run `timely auth status` or set TIMELY_TOKEN / TIMELY_ACCOUNT_ID."
        );
    }
}
