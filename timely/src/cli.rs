use clap::{Parser, Subcommand, ValueEnum};

use crate::api_cli::ApiCommand;

pub use crate::cli_commands::{
    AuthCommand, AuthSubcommand, CallCommand, DEFAULT_BASE_URL, DEFAULT_OAUTH_REDIRECT_URI,
    DEFAULT_PROFILE, HttpVerb, McpCommand, McpSubcommand, OauthCommand, RequestCommand,
    SecretProvider, SourceCommand, SpecCommand, SpecSubcommand,
};
pub use crate::cli_config::{ConfigCommand, ConfigSubcommand};
pub use crate::cli_memory::{MemoryCommand, MemorySubcommand};

pub const LONG_ABOUT: &str = "\
Query Timely accounts, projects, time entries, and more from the terminal.

Examples:
  timely auth status
  timely config set oauth.client_id YOUR_CLIENT_ID
  timely api me
  timely api clients list
  timely call listClients --account-id 123
  echo '[{\"args\":[\"spec\",\"summary\"]}]' | timely batch

Documentation: https://github.com/amkisko/timely-cli.rs
Report issues: https://github.com/amkisko/timely-cli.rs/issues";

pub const AFTER_HELP: &str = "\
Output (default: human tables on a TTY, compact JSON when piped):
  -o auto           TTY → tables; non-TTY → compact JSON (default)
  -o plain          Human-readable tables
  --plain           Script-stable tab-separated records
  -o json           Pretty JSON
  --json            Compact JSON for scripts
  --json-pretty     Indented JSON

Batch (`timely batch`): stdout is always a JSON report; use --json-pretty for indented output.

Destructive API changes (delete/archive) prompt on a TTY, or require --yes when piped.
Use --dry-run to print the planned action without calling the API.

Exit codes: 0 success, 1 general error, 2 usage, 3 auth, 4 API, 5 I/O

Run `timely help <command>` for command-specific examples.";

#[derive(Parser, Debug)]
#[command(
    name = "timely",
    bin_name = "timely",
    version,
    about = "Timely API CLI, MCP server, and local Memory reader",
    long_about = LONG_ABOUT,
    after_help = AFTER_HELP
)]
pub struct Cli {
    #[arg(
        long,
        env = "TIMELY_PROFILE",
        default_value = DEFAULT_PROFILE,
        global = true,
        help = "Credential profile name"
    )]
    pub profile: String,
    #[arg(
        long,
        env = "TIMELY_BASE_URL",
        default_value = DEFAULT_BASE_URL,
        global = true,
        help = "Timely API base URL"
    )]
    pub base_url: String,

    #[arg(
        short,
        long,
        global = true,
        default_value = "auto",
        value_enum,
        env = "TIMELY_OUTPUT",
        help = "Output format: auto, plain, or json"
    )]
    pub output: OutputFormatArg,

    #[arg(long, global = true, help = "Compact JSON on stdout")]
    pub json: bool,

    #[arg(
        long,
        global = true,
        conflicts_with = "json",
        help = "Script-stable tab-separated records"
    )]
    pub plain: bool,

    #[arg(
        long,
        global = true,
        conflicts_with_all = ["json", "plain"],
        help = "Pretty-printed JSON on stdout"
    )]
    pub json_pretty: bool,

    #[arg(
        short,
        long,
        global = true,
        help = "Suppress progress and hints on stderr"
    )]
    pub quiet: bool,

    #[arg(
        short,
        long,
        global = true,
        env = "TIMELY_DEBUG",
        help = "Show debug details"
    )]
    pub debug: bool,

    #[arg(long, global = true, help = "Show extra error details")]
    pub verbose: bool,

    #[arg(
        long,
        global = true,
        env = "TIMELY_NO_COLOR",
        help = "Disable ANSI colors"
    )]
    pub no_color: bool,

    #[arg(
        long,
        global = true,
        env = "TIMELY_TIMEOUT",
        help = "HTTP timeout in seconds"
    )]
    pub timeout: Option<u64>,

    #[arg(
        short = 'y',
        long,
        global = true,
        help = "Confirm destructive API changes without prompting"
    )]
    pub yes: bool,

    #[arg(
        long,
        global = true,
        help = "Show destructive actions without calling the API"
    )]
    pub dry_run: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Clone, Copy, ValueEnum, Debug, PartialEq, Eq)]
pub enum OutputFormatArg {
    Auto,
    Plain,
    Json,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(
        about = "Inspect the vendored OpenAPI document",
        next_help_heading = "OpenAPI"
    )]
    Spec(SpecCommand),

    #[command(
        about = "Manage authentication and credentials",
        next_help_heading = "Authentication"
    )]
    Auth(AuthCommand),

    #[command(
        about = "Manage home config (TIMELY_HOME / config.env)",
        next_help_heading = "Configuration"
    )]
    Config(ConfigCommand),

    #[command(
        about = "Call curated Timely API endpoints",
        next_help_heading = "Timely API"
    )]
    Api(Box<ApiCommand>),

    #[command(
        about = "Call an OpenAPI operation by operationId",
        next_help_heading = "OpenAPI"
    )]
    Call(CallCommand),

    #[command(
        about = "Send a raw authenticated HTTP request",
        next_help_heading = "OpenAPI"
    )]
    Request(RequestCommand),

    #[command(about = "Read the local Memory database", next_help_heading = "Local")]
    Memory(MemoryCommand),

    #[command(about = "Run the MCP server over stdio", next_help_heading = "MCP")]
    Mcp(McpCommand),

    #[command(
        about = "Generate shell completion scripts",
        next_help_heading = "Utilities"
    )]
    Completions {
        shell: crate::completions::CompletionShell,
    },

    #[command(
        about = "Print a man page (roff) to stdout",
        next_help_heading = "Utilities"
    )]
    Man,

    #[command(about = "Print the CLI version", next_help_heading = "Utilities")]
    Version,

    #[command(
        about = "Run multiple timely operations from a JSON plan",
        long_about = "Run multiple timely operations from a JSON plan. \
                      Output is always a JSON report on stdout.",
        after_help = "Plan format: JSON array of {\"id\":\"optional\",\"args\":[\"subcommand\",...]} \
                      or {\"operations\":[...]}.\n\n\
                      Not allowed inside batch: nested batch, completions, man, version, \
                      auth token/logout/oauth/source, config set/unset, mcp serve.\n\n\
                      Examples:\n  \
                      echo '[{\"args\":[\"spec\",\"summary\"]}]' | timely batch\n  \
                      timely batch --file plan.json\n  \
                      timely batch --fail-fast --file plan.json",
        next_help_heading = "Utilities"
    )]
    Batch {
        #[arg(
            long,
            value_name = "FILE",
            help = "Read plan from file, or - for stdin"
        )]
        file: Option<String>,
        #[arg(long, help = "Stop after the first failed operation")]
        fail_fast: bool,
    },
}
