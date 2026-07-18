//! Shared runtime context for command handlers.

use crate::cli_error::ErrorContext;
use crate::confirm::ConfirmOptions;
use crate::output::EmitOptions;

pub struct RunContext {
    pub emit: EmitOptions,
    pub error_context: ErrorContext,
    pub confirm: ConfirmOptions,
}
