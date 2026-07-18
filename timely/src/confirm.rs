//! Confirmation for destructive remote API mutations.

use std::io::{self, Write};

use anyhow::{Result, bail};

use crate::cli_util;

#[derive(Debug, Clone, Copy, Default)]
pub struct ConfirmOptions {
    pub yes: bool,
    pub dry_run: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardResult {
    Proceed,
    DryRun,
}

pub fn guard(action: &str, options: &ConfirmOptions) -> Result<GuardResult> {
    if options.dry_run {
        cli_util::user_notice(false, &format!("Dry run: would {action}"));
        return Ok(GuardResult::DryRun);
    }
    if options.yes {
        return Ok(GuardResult::Proceed);
    }
    if !cli_util::stdin_is_tty() {
        bail!("refusing to {action} without confirmation; pass --yes to confirm");
    }
    eprint!("About to {action}. Continue? [y/N] ");
    io::stderr().flush()?;
    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    let answer = answer.trim().to_ascii_lowercase();
    if answer == "y" || answer == "yes" {
        Ok(GuardResult::Proceed)
    } else {
        bail!("cancelled; pass --yes to confirm without a prompt");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dry_run_skips_confirmation() {
        assert_eq!(
            guard(
                "delete time entry 1",
                &ConfirmOptions {
                    yes: false,
                    dry_run: true,
                }
            )
            .unwrap(),
            GuardResult::DryRun
        );
    }

    #[test]
    fn yes_proceeds_without_prompt() {
        assert_eq!(
            guard(
                "delete time entry 1",
                &ConfirmOptions {
                    yes: true,
                    dry_run: false,
                }
            )
            .unwrap(),
            GuardResult::Proceed
        );
    }
}
