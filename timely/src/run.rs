//! CLI entrypoint and command dispatch.

use clap::Parser;
use std::process::ExitCode;

use crate::api_commands;
use crate::api_suggest;
use crate::auth;
use crate::batch_cmd;
use crate::cli::{Cli, Commands, McpSubcommand, OutputFormatArg};
use crate::cli_error::{self, ErrorContext};
use crate::cli_util;
use crate::commands;
use crate::completions;
use crate::config_cmd;
use crate::confirm::ConfirmOptions;
use crate::exit::AppExit;
use crate::man;
use crate::mcp;
use crate::memory;
use crate::output::{self, OutputFormat, OutputMode};
use crate::run_context::RunContext;
use timely_lib::Api;
use timely_lib::TimelyError;
use timely_lib::ensure_home_config_loaded;

pub fn run() -> ExitCode {
    match try_run() {
        Ok(()) => AppExit::Success.code(),
        Err(exit) => exit.code(),
    }
}

fn try_run() -> Result<(), AppExit> {
    ensure_home_config_loaded();
    let cli = Cli::parse();
    let error_context = error_context(&cli);

    if matches!(cli.command, Commands::Version) {
        println!("timely {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if let Commands::Completions { shell } = cli.command {
        return completions::run(shell)
            .map_err(|error| cli_error::print_error(&error, &error_context));
    }

    if matches!(cli.command, Commands::Man) {
        return man::run().map_err(|error| cli_error::print_error(&error, &error_context));
    }

    if matches!(&cli.command, Commands::Config(_)) {
        let run_context = RunContext {
            emit: emit_options(&cli),
            error_context: error_context.clone(),
            confirm: ConfirmOptions {
                yes: cli.yes,
                dry_run: cli.dry_run,
            },
        };
        let Commands::Config(cmd) = cli.command else {
            unreachable!("checked above")
        };
        return match config_cmd::run_config(cmd, &run_context) {
            Ok(Some(value)) => emit_config_result(value, &run_context),
            Ok(None) => Ok(()),
            Err(error) => Err(cli_error::print_error(&error, &error_context)),
        };
    }

    if let Commands::Batch {
        ref file,
        fail_fast,
    } = cli.command
    {
        let operations = match batch_cmd::load_operations(file.as_deref()) {
            Err(batch_cmd::BatchLoadError::MissingInteractiveInput) => {
                batch_cmd::print_concise_usage();
                return Err(AppExit::Usage);
            }
            Err(batch_cmd::BatchLoadError::Invalid(message)) => {
                return Err(cli_error::print_error(&message, &error_context));
            }
            Ok(operations) => operations,
        };
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|error| cli_error::print_error(&error.to_string(), &error_context))?;
        return runtime.block_on(batch_cmd::run_batch(cli, operations, fail_fast));
    }

    let runtime = tokio::runtime::Runtime::new()
        .map_err(|error| cli_error::print_error(&error.to_string(), &error_context))?;
    runtime.block_on(run_async(cli, error_context))
}

async fn run_async(cli: Cli, error_context: ErrorContext) -> Result<(), AppExit> {
    let api = Api::new(cli.profile.clone(), cli.base_url.clone(), cli.timeout);
    let run_context = RunContext {
        emit: emit_options(&cli),
        error_context: error_context.clone(),
        confirm: ConfirmOptions {
            yes: cli.yes,
            dry_run: cli.dry_run,
        },
    };

    match cli.command {
        Commands::Spec(cmd) => emit_result(
            commands::spec_command_value(cmd).map_err(TimelyError::from_anyhow),
            &run_context,
        ),
        Commands::Auth(cmd) => match auth::run_auth(&api, cmd).await {
            Ok(Some(value)) => emit_result(Ok(value), &run_context),
            Ok(None) => Ok(()),
            Err(error) => Err(cli_error::print_timely_error(
                &TimelyError::from_anyhow(error),
                &error_context,
            )),
        },
        Commands::Api(api_cmd) => {
            let hint = api_suggest::list_hint(&api_cmd.command);
            if !cli.dry_run {
                cli_util::progress_message(cli.quiet, "Calling Timely API…");
            }
            let result = api_commands::api_command_value(&api, *api_cmd, &run_context.confirm)
                .await
                .map_err(TimelyError::from_anyhow);
            if let (Some(hint), Ok(value)) = (hint, &result) {
                api_suggest::maybe_suggest(hint, value, cli.quiet);
            }
            emit_result(result, &run_context)
        }
        Commands::Call(cmd) => emit_result(
            commands::call_command_value(&api, cmd)
                .await
                .map_err(TimelyError::from_anyhow),
            &run_context,
        ),
        Commands::Request(cmd) => emit_result(
            commands::request_command_value(&api, cmd)
                .await
                .map_err(TimelyError::from_anyhow),
            &run_context,
        ),
        Commands::Memory(cmd) => match memory::run_memory(cmd) {
            Ok(Some(value)) => emit_result(Ok(value), &run_context),
            Ok(None) => Ok(()),
            Err(error) => Err(cli_error::print_timely_error(
                &TimelyError::from_anyhow(error),
                &error_context,
            )),
        },
        Commands::Mcp(cmd) => match cmd.command {
            McpSubcommand::Serve => mcp::run(api).await.map_err(|error| {
                cli_error::print_timely_error(&TimelyError::from_anyhow(error), &error_context)
            }),
        },
        Commands::Completions { .. }
        | Commands::Man
        | Commands::Version
        | Commands::Config(_)
        | Commands::Batch { .. } => unreachable!("handled before async dispatch"),
    }
}

fn emit_result(
    result: Result<serde_json::Value, TimelyError>,
    context: &RunContext,
) -> Result<(), AppExit> {
    let value =
        result.map_err(|error| cli_error::print_timely_error(&error, &context.error_context))?;
    output::emit_value(context.emit, &value)
        .map_err(|error| cli_error::print_error(&error, &context.error_context))
}

fn emit_config_result(value: serde_json::Value, context: &RunContext) -> Result<(), AppExit> {
    match context.emit.mode {
        OutputMode::HumanPlain | OutputMode::ScriptPlain => {
            if let Some(text) = config_cmd::format_plain_config(&value) {
                println!("{text}");
                return Ok(());
            }
        }
        OutputMode::JsonCompact | OutputMode::JsonPretty => {}
    }
    output::emit_value(context.emit, &value)
        .map_err(|error| cli_error::print_error(&error, &context.error_context))
}

fn emit_options(cli: &Cli) -> output::EmitOptions {
    output::EmitOptions {
        mode: resolve_output_mode(cli),
        use_color: cli_util::color_enabled(cli.no_color),
    }
}

fn resolve_output_mode(cli: &Cli) -> OutputMode {
    let output = match cli.output {
        OutputFormatArg::Auto => OutputFormat::Auto,
        OutputFormatArg::Plain => OutputFormat::Plain,
        OutputFormatArg::Json => OutputFormat::Json,
    };
    output::resolve_output_mode(
        output,
        cli.json,
        cli.plain,
        cli.json_pretty,
        cli_util::stdout_is_tty(),
    )
}

fn error_context(cli: &Cli) -> ErrorContext {
    ErrorContext {
        quiet: cli.quiet,
        verbose: cli.verbose,
        debug: cli_util::debug_enabled(cli.debug),
    }
}
