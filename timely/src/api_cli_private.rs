use clap::{Args, Subcommand};

use crate::api_cli::ApiTarget;
use crate::cli::HttpVerb;

#[derive(Args, Debug)]
pub struct ExperimentalCommand {
    #[command(subcommand)]
    pub command: ExperimentalSubcommand,
}

#[derive(Subcommand, Debug)]
pub enum ExperimentalSubcommand {
    Memory(ExperimentalMemoryCommand),
}

#[derive(Args, Debug)]
pub struct ExperimentalMemoryCommand {
    #[command(subcommand)]
    pub command: ExperimentalMemorySubcommand,
}

#[derive(Subcommand, Debug)]
pub enum ExperimentalMemorySubcommand {
    Accounts(ApiTarget),
    Identity(ApiTarget),
    LinkedEntries(ExperimentalLinkedEntriesCommand),
    Request(ExperimentalRequestCommand),
}

#[derive(Args, Debug)]
pub struct ExperimentalLinkedEntriesCommand {
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
    pub all_users: bool,
    #[arg(long)]
    pub page: Option<i64>,
    #[arg(long)]
    pub per_page: Option<i64>,
}

#[derive(Args, Debug)]
pub struct ExperimentalRequestCommand {
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
