use clap::{Args, Subcommand};

use crate::api_cli_extra::{PermissionsCommand, ProjectCommand, ReportsCommand, TeamCommand};
use crate::api_cli_private::ExperimentalCommand;
use crate::cli::HttpVerb;

#[derive(Args, Debug)]
pub struct ApiCommand {
    #[command(subcommand)]
    pub command: ApiSubcommand,
}

#[derive(Subcommand, Debug)]
pub enum ApiSubcommand {
    /// Current authenticated user
    Me(ApiTarget),
    /// Client resources
    Clients(ResourceCommand),
    /// Team resources
    Teams(TeamCommand),
    /// Project resources
    Projects(ProjectCommand),
    /// User resources
    Users(ResourceCommand),
    /// Label resources
    Labels(ResourceCommand),
    /// Task resources
    Tasks(ResourceCommand),
    /// Permission resources
    Permissions(PermissionsCommand),
    /// Undocumented Memory API helpers
    Experimental(ExperimentalCommand),
    /// Report endpoints
    Reports(ReportsCommand),
    /// Time entry resources and timers
    TimeEntries(TimeEntriesCommand),
    /// Raw authenticated HTTP request
    Raw(ApiRawCommand),
}

#[derive(Args, Debug, Clone, Default)]
pub struct ApiTarget {
    #[arg(long)]
    pub account_id: Option<i64>,
}

#[derive(Args, Debug)]
pub struct ResourceCommand {
    #[command(subcommand)]
    pub command: ResourceSubcommand,
}

#[derive(Subcommand, Debug)]
pub enum ResourceSubcommand {
    List(QueryListCommand),
    Get(ResourceGetCommand),
}

#[derive(Args, Debug)]
pub struct QueryListCommand {
    #[command(flatten)]
    pub target: ApiTarget,
    #[arg(long = "query")]
    pub query: Vec<String>,
}

#[derive(Args, Debug)]
pub struct ResourceGetCommand {
    pub id: i64,
    #[command(flatten)]
    pub target: ApiTarget,
}

#[derive(Args, Debug)]
pub struct TimeEntriesCommand {
    #[command(subcommand)]
    pub command: TimeEntriesSubcommand,
}

#[derive(Subcommand, Debug)]
pub enum TimeEntriesSubcommand {
    /// List time entries
    List(TimeEntryListCommand),
    /// Get a time entry by id
    Get(ResourceGetCommand),
    /// Create a time entry
    Create(TimeEntryCreateCommand),
    /// Update a time entry
    Update(TimeEntryUpdateCommand),
    /// Delete a time entry
    Delete(ResourceGetCommand),
    /// Start a timer on a time entry
    Start(ResourceGetCommand),
    /// Stop a timer on a time entry
    Stop(ResourceGetCommand),
}

#[derive(Args, Debug)]
pub struct TimeEntryListCommand {
    #[command(flatten)]
    pub target: ApiTarget,
    #[arg(long = "query")]
    pub query: Vec<String>,
    #[arg(long)]
    pub since: Option<String>,
    #[arg(long)]
    pub upto: Option<String>,
    #[arg(long)]
    pub day: Option<String>,
    #[arg(long)]
    pub user_id: Option<i64>,
    #[arg(long)]
    pub project_id: Option<i64>,
    #[arg(long)]
    pub per_page: Option<i64>,
    #[arg(long)]
    pub page: Option<i64>,
    #[arg(long)]
    pub sort: Option<String>,
    #[arg(long)]
    pub order: Option<String>,
    #[arg(long)]
    pub all_users: bool,
    #[arg(long)]
    pub include_linked_metadata: bool,
}

#[derive(Args, Debug)]
pub struct TimeEntryCreateCommand {
    #[command(flatten)]
    pub target: ApiTarget,
    #[arg(long)]
    pub project_id: i64,
    #[arg(long)]
    pub day: String,
    #[arg(long)]
    pub hours: Option<f64>,
    #[arg(long)]
    pub minutes: Option<i64>,
    #[arg(long)]
    pub seconds: Option<i64>,
    #[arg(long)]
    pub note: Option<String>,
    #[arg(long)]
    pub from: Option<String>,
    #[arg(long)]
    pub to: Option<String>,
    #[arg(long = "label-id")]
    pub label_ids: Vec<i64>,
}

#[derive(Args, Debug)]
pub struct TimeEntryUpdateCommand {
    pub id: i64,
    #[command(flatten)]
    pub target: ApiTarget,
    #[arg(long)]
    pub project_id: Option<i64>,
    #[arg(long)]
    pub day: Option<String>,
    #[arg(long)]
    pub hours: Option<f64>,
    #[arg(long)]
    pub minutes: Option<i64>,
    #[arg(long)]
    pub seconds: Option<i64>,
    #[arg(long)]
    pub note: Option<String>,
    #[arg(long)]
    pub from: Option<String>,
    #[arg(long)]
    pub to: Option<String>,
    #[arg(long = "label-id")]
    pub label_ids: Vec<i64>,
}

#[derive(Args, Debug)]
pub struct ApiRawCommand {
    #[command(flatten)]
    pub target: ApiTarget,
    #[arg(value_enum)]
    pub method: HttpVerb,
    pub path: String,
    #[arg(long = "query")]
    pub query: Vec<String>,
    #[arg(long)]
    pub body: Option<String>,
    #[arg(long = "body-file")]
    pub body_file: Option<String>,
}
