//! Run multiple timely operations in one invocation.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

use timely_lib::{Api, TimelyError};

use crate::batch_exit::{BatchExitOutcome, exit_for_batch_outcomes};
use crate::cli::Cli;
use crate::cli_error::ErrorContext;
use crate::cli_util;
use crate::confirm::ConfirmOptions;
use crate::execute::{self, format_operation_error, parse_operation_cli};
use crate::exit::{AppExit, exit_for_timely_error};
use crate::output::{self, OutputMode};
use crate::run_context::RunContext;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BatchLoadError {
    MissingInteractiveInput,
    Invalid(String),
}

#[derive(Debug, Clone, Deserialize)]
pub struct BatchOperation {
    #[serde(default)]
    pub id: Option<String>,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BatchItemResult {
    pub index: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub args: Vec<String>,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
struct BatchItemOutcome {
    result: BatchItemResult,
    exit: Option<AppExit>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BatchReport {
    pub operations: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub results: Vec<BatchItemResult>,
}

pub fn print_concise_usage() {
    cli_util::print_concise_usage(
        Some("batch"),
        "Run multiple timely operations from a JSON plan. \
         Output is always a JSON report on stdout.",
        &[
            "echo '[{\"args\":[\"spec\",\"summary\"]}]' | timely batch",
            "timely batch --file plan.json",
            "timely batch --help",
        ],
    );
}

pub fn load_operations(file: Option<&str>) -> Result<Vec<BatchOperation>, BatchLoadError> {
    let text = match file {
        Some("-") | None if !cli_util::stdin_is_tty() => read_stdin_text()?,
        Some(path) => std::fs::read_to_string(Path::new(path))
            .map_err(|error| BatchLoadError::Invalid(format!("read batch file: {error}")))?,
        None => return Err(BatchLoadError::MissingInteractiveInput),
    };
    parse_operations_json(&text)
}

fn read_stdin_text() -> Result<String, BatchLoadError> {
    use std::io::Read;
    let mut buffer = String::new();
    std::io::stdin()
        .read_to_string(&mut buffer)
        .map_err(|error| BatchLoadError::Invalid(format!("read stdin: {error}")))?;
    if buffer.trim().is_empty() {
        return Err(BatchLoadError::Invalid("batch stdin is empty".to_string()));
    }
    Ok(buffer)
}

fn parse_operations_json(text: &str) -> Result<Vec<BatchOperation>, BatchLoadError> {
    if let Ok(operations) = serde_json::from_str::<Vec<BatchOperation>>(text) {
        return validate_operations(operations);
    }
    if let Ok(wrapper) = serde_json::from_str::<BatchFile>(text) {
        return validate_operations(wrapper.operations);
    }
    Err(BatchLoadError::Invalid(
        "batch input must be a JSON array or {\"operations\":[...]}".to_string(),
    ))
}

#[derive(Debug, Deserialize)]
struct BatchFile {
    operations: Vec<BatchOperation>,
}

fn validate_operations(
    operations: Vec<BatchOperation>,
) -> Result<Vec<BatchOperation>, BatchLoadError> {
    if operations.is_empty() {
        return Err(BatchLoadError::Invalid(
            "batch must include at least one operation".to_string(),
        ));
    }
    for (index, operation) in operations.iter().enumerate() {
        if operation.args.is_empty() {
            return Err(BatchLoadError::Invalid(format!(
                "operation {index} is missing args"
            )));
        }
    }
    Ok(operations)
}

pub fn batch_output_mode(cli: &Cli) -> OutputMode {
    if cli.json_pretty || matches!(cli.output, crate::cli::OutputFormatArg::Json) {
        OutputMode::JsonPretty
    } else {
        OutputMode::JsonCompact
    }
}

fn batch_needs_api(parent: &Cli, operations: &[BatchOperation]) -> bool {
    operations.iter().any(|operation| {
        parse_operation_cli(parent, &operation.args)
            .ok()
            .map(|cli| execute::command_requires_api(&cli.command))
            .unwrap_or(false)
    })
}

pub async fn run_batch(
    parent: Cli,
    operations: Vec<BatchOperation>,
    fail_fast: bool,
) -> Result<(), AppExit> {
    let needs_api = batch_needs_api(&parent, &operations);
    let output_mode = batch_output_mode(&parent);
    let planned_total = operations.len();
    let error_context = error_context(&parent);

    let api = if needs_api {
        cli_util::progress_message(parent.quiet, "Preparing Timely API client…");
        Api::new(
            parent.profile.clone(),
            parent.base_url.clone(),
            parent.timeout,
        )
    } else {
        Api::new(
            parent.profile.clone(),
            parent.base_url.clone(),
            parent.timeout,
        )
    };

    let run_context = RunContext {
        emit: output::EmitOptions {
            mode: output_mode,
            use_color: false,
        },
        error_context: error_context.clone(),
        confirm: ConfirmOptions {
            yes: parent.yes,
            dry_run: parent.dry_run,
        },
    };

    let total = operations.len();
    let mut outcomes = Vec::with_capacity(total);

    for (index, operation) in operations.into_iter().enumerate() {
        let label = operation
            .id
            .clone()
            .unwrap_or_else(|| operation.args.join(" "));
        cli_util::progress_message(
            parent.quiet,
            &format!("Batch {}/{}: {label}", index + 1, total),
        );

        let outcome = match parse_operation_cli(&parent, &operation.args) {
            Ok(parsed) => {
                match execute::execute_command_value(&api, parsed.command, &run_context).await {
                    Ok(data) => BatchItemOutcome {
                        result: BatchItemResult {
                            index,
                            id: operation.id,
                            args: operation.args,
                            ok: true,
                            data: Some(data),
                            error: None,
                        },
                        exit: None,
                    },
                    Err(error) => failed_outcome(index, operation, error),
                }
            }
            Err(error) => failed_outcome(index, operation, error),
        };
        let operation_failed = !outcome.result.ok;
        outcomes.push(outcome);
        if fail_fast && operation_failed {
            break;
        }
    }

    let results: Vec<BatchItemResult> = outcomes
        .iter()
        .map(|outcome| outcome.result.clone())
        .collect();
    let succeeded = results.iter().filter(|item| item.ok).count();
    let failed = results.len() - succeeded;
    let attempted = results.len();
    let report = BatchReport {
        operations: results.len(),
        succeeded,
        failed,
        results: results.clone(),
    };

    let value = serde_json::to_value(&report)
        .map_err(|error| crate::cli_error::print_error(&error.to_string(), &error_context))?;
    output::emit_value(
        output::EmitOptions {
            mode: output_mode,
            use_color: false,
        },
        &value,
    )
    .map_err(|error| crate::cli_error::print_error(&error, &error_context))?;

    if failed == 0 {
        return Ok(());
    }
    if !parent.quiet {
        let notice = if fail_fast && attempted < planned_total {
            format!("Stopped after first failure ({attempted} of {planned_total} operations run).")
        } else {
            format!(
                "{failed} of {attempted} operations failed — see results[].error in the report."
            )
        };
        cli_util::user_notice(false, &notice);
    }
    let exits: Vec<_> = outcomes
        .iter()
        .map(|outcome| BatchExitOutcome { exit: outcome.exit })
        .collect();
    Err(exit_for_batch_outcomes(&exits))
}

fn failed_outcome(index: usize, operation: BatchOperation, error: TimelyError) -> BatchItemOutcome {
    BatchItemOutcome {
        result: BatchItemResult {
            index,
            id: operation.id,
            args: operation.args,
            ok: false,
            data: None,
            error: Some(format_operation_error(&error)),
        },
        exit: Some(exit_for_timely_error(&error)),
    }
}

fn error_context(parent: &Cli) -> ErrorContext {
    ErrorContext {
        quiet: parent.quiet,
        verbose: parent.verbose,
        debug: cli_util::debug_enabled(parent.debug),
    }
}
