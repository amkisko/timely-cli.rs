//! Aggregate exit codes for batch operation outcomes.

use crate::exit::AppExit;

#[derive(Debug, Clone)]
pub(crate) struct BatchExitOutcome {
    pub exit: Option<AppExit>,
}

pub(crate) fn exit_for_batch_outcomes(outcomes: &[BatchExitOutcome]) -> AppExit {
    outcomes
        .iter()
        .filter_map(|outcome| outcome.exit)
        .max_by_key(|exit| exit_severity(*exit))
        .unwrap_or(AppExit::General)
}

fn exit_severity(exit: AppExit) -> u8 {
    match exit {
        AppExit::Success => 0,
        AppExit::General => 1,
        AppExit::Usage => 2,
        AppExit::Io => 3,
        AppExit::Api => 4,
        AppExit::Auth => 5,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outcome(exit: Option<AppExit>) -> BatchExitOutcome {
        BatchExitOutcome { exit }
    }

    #[test]
    fn batch_exit_prefers_auth_over_usage() {
        let exit =
            exit_for_batch_outcomes(&[outcome(Some(AppExit::Usage)), outcome(Some(AppExit::Auth))]);
        assert_eq!(exit, AppExit::Auth);
    }

    #[test]
    fn batch_exit_uses_usage_for_plan_validation_failures() {
        let exit = exit_for_batch_outcomes(&[outcome(Some(AppExit::Usage))]);
        assert_eq!(exit, AppExit::Usage);
    }
}
