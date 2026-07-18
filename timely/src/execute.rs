//! Run a single timely command and return structured data (for batch and scripting).

use clap::Parser;
use serde_json::Value;

use timely_lib::{Api, TimelyError};

use crate::api_commands;
use crate::auth;
use crate::cli::{AuthSubcommand, Commands, ConfigSubcommand, McpCommand, McpSubcommand};
use crate::cli_commands::AuthCommand as AuthArgs;
use crate::cli_config::ConfigCommand as ConfigArgs;
use crate::commands;
use crate::config_cmd;
use crate::memory;
use crate::run_context::RunContext;

pub fn command_requires_api(command: &Commands) -> bool {
    match command {
        Commands::Spec(_)
        | Commands::Auth(AuthArgs {
            command: AuthSubcommand::Status | AuthSubcommand::Export { .. },
        })
        | Commands::Config(_)
        | Commands::Memory(_)
        | Commands::Completions { .. }
        | Commands::Man
        | Commands::Version
        | Commands::Batch { .. } => false,
        Commands::Mcp(_) => true,
        Commands::Auth(_) | Commands::Api(_) | Commands::Call(_) | Commands::Request(_) => true,
    }
}

pub fn command_allowed_in_batch(command: &Commands) -> Result<(), TimelyError> {
    match command {
        Commands::Completions { .. } => Err(TimelyError::Usage(
            "completions cannot run inside batch".to_string(),
        )),
        Commands::Man => Err(TimelyError::Usage(
            "man cannot run inside batch".to_string(),
        )),
        Commands::Version => Err(TimelyError::Usage(
            "use batch only for data commands".to_string(),
        )),
        Commands::Batch { .. } => Err(TimelyError::Usage(
            "nested batch is not supported".to_string(),
        )),
        Commands::Mcp(McpCommand {
            command: McpSubcommand::Serve,
        }) => Err(TimelyError::Usage(
            "mcp serve cannot run inside batch (long-running)".to_string(),
        )),
        Commands::Auth(AuthArgs {
            command:
                AuthSubcommand::Token { .. }
                | AuthSubcommand::Logout
                | AuthSubcommand::Source(_)
                | AuthSubcommand::Oauth(_),
        }) => Err(TimelyError::Usage(
            "auth token/logout/oauth/source cannot run inside batch (state-changing)".to_string(),
        )),
        Commands::Config(ConfigArgs {
            command: ConfigSubcommand::Set { .. } | ConfigSubcommand::Unset { .. },
        }) => Err(TimelyError::Usage(
            "config set/unset cannot run inside batch (state-changing)".to_string(),
        )),
        _ => Ok(()),
    }
}

pub async fn execute_command_value(
    api: &Api,
    command: Commands,
    context: &RunContext,
) -> Result<Value, TimelyError> {
    command_allowed_in_batch(&command)?;
    match command {
        Commands::Spec(cmd) => commands::spec_command_value(cmd).map_err(TimelyError::from_anyhow),
        Commands::Auth(cmd) => auth::auth_command_value(api, cmd).map_err(TimelyError::from_anyhow),
        Commands::Api(cmd) => api_commands::api_command_value(api, *cmd, &context.confirm)
            .await
            .map_err(TimelyError::from_anyhow),
        Commands::Call(cmd) => commands::call_command_value(api, cmd)
            .await
            .map_err(TimelyError::from_anyhow),
        Commands::Request(cmd) => commands::request_command_value(api, cmd)
            .await
            .map_err(TimelyError::from_anyhow),
        Commands::Memory(cmd) => {
            memory::memory_command_value(cmd).map_err(TimelyError::from_anyhow)
        }
        Commands::Config(cmd) => config_cmd::config_command_value(cmd).map_err(TimelyError::Usage),
        other => Err(TimelyError::Usage(format!(
            "unsupported batch command: {other:?}"
        ))),
    }
}

pub fn build_operation_argv(parent: &crate::cli::Cli, operation_args: &[String]) -> Vec<String> {
    let mut argv = vec!["timely".to_string()];
    argv.push("--profile".to_string());
    argv.push(parent.profile.clone());
    argv.push("--base-url".to_string());
    argv.push(parent.base_url.clone());
    if parent.quiet {
        argv.push("--quiet".to_string());
    }
    if parent.json {
        argv.push("--json".to_string());
    }
    if parent.plain {
        argv.push("--plain".to_string());
    }
    if parent.json_pretty {
        argv.push("--json-pretty".to_string());
    }
    if parent.no_color {
        argv.push("--no-color".to_string());
    }
    if let Some(timeout) = parent.timeout {
        argv.push("--timeout".to_string());
        argv.push(timeout.to_string());
    }
    if parent.yes {
        argv.push("--yes".to_string());
    }
    if parent.dry_run {
        argv.push("--dry-run".to_string());
    }
    argv.extend(operation_args.iter().cloned());
    argv
}

pub fn parse_operation_cli(
    parent: &crate::cli::Cli,
    operation_args: &[String],
) -> Result<crate::cli::Cli, TimelyError> {
    let binding = build_operation_argv(parent, operation_args);
    let argv: Vec<&str> = binding.iter().map(String::as_str).collect();
    crate::cli::Cli::try_parse_from(argv).map_err(|error| TimelyError::Usage(error.to_string()))
}

pub fn format_operation_error(error: &TimelyError) -> String {
    let rendered = error.to_string();
    rendered
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or(rendered.as_str())
        .trim()
        .to_string()
}
