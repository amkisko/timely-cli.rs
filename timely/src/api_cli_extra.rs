use clap::{Args, Subcommand};

use crate::api_cli::{ApiTarget, QueryListCommand, ResourceGetCommand};

#[derive(Args, Debug)]
pub struct TeamCommand {
    #[command(subcommand)]
    pub command: TeamSubcommand,
}

#[derive(Subcommand, Debug)]
pub enum TeamSubcommand {
    /// List teams
    List(QueryListCommand),
    /// Get a team by id
    Get(ResourceGetCommand),
    /// Search teams
    Search(TeamSearchCommand),
    /// Create a team
    Create(TeamCreateCommand),
    /// Update a team
    Update(TeamUpdateCommand),
    /// Delete a team
    Delete(ResourceGetCommand),
}

#[derive(Args, Debug)]
pub struct TeamSearchCommand {
    #[command(flatten)]
    pub target: ApiTarget,
    #[arg(long)]
    pub query: String,
    #[arg(long)]
    pub per_page: Option<i64>,
    #[arg(long)]
    pub page: Option<i64>,
}

#[derive(Args, Debug)]
pub struct TeamCreateCommand {
    #[command(flatten)]
    pub target: ApiTarget,
    #[arg(long)]
    pub name: String,
    #[arg(long)]
    pub color: Option<String>,
    #[arg(long)]
    pub emoji: Option<String>,
    #[arg(long = "user-id")]
    pub user_ids: Vec<i64>,
    #[arg(long = "lead-user-id")]
    pub lead_user_ids: Vec<i64>,
    #[arg(long = "hide-hours-user-id")]
    pub hide_hours_user_ids: Vec<i64>,
}

#[derive(Args, Debug)]
pub struct TeamUpdateCommand {
    pub id: i64,
    #[command(flatten)]
    pub target: ApiTarget,
    #[arg(long)]
    pub name: Option<String>,
    #[arg(long)]
    pub color: Option<String>,
    #[arg(long)]
    pub emoji: Option<String>,
    #[arg(long = "user-id")]
    pub user_ids: Vec<i64>,
    #[arg(long = "lead-user-id")]
    pub lead_user_ids: Vec<i64>,
    #[arg(long = "hide-hours-user-id")]
    pub hide_hours_user_ids: Vec<i64>,
    #[arg(long)]
    pub add_users_to_team_projects: Option<bool>,
    #[arg(long)]
    pub delete_users_from_team_projects: Option<bool>,
}

#[derive(Args, Debug)]
pub struct ProjectCommand {
    #[command(subcommand)]
    pub command: ProjectSubcommand,
}

#[derive(Subcommand, Debug)]
pub enum ProjectSubcommand {
    /// List projects
    List(QueryListCommand),
    /// Get a project by id
    Get(ResourceGetCommand),
    /// Create a project
    Create(ProjectCreateCommand),
    /// Update a project
    Update(ProjectUpdateCommand),
    /// Delete a project
    Delete(ResourceGetCommand),
    /// Archive a project
    Archive(ResourceGetCommand),
    /// Unarchive a project
    Unarchive(ResourceGetCommand),
}

#[derive(Args, Debug)]
pub struct PermissionsCommand {
    #[command(subcommand)]
    pub command: PermissionsSubcommand,
}

#[derive(Subcommand, Debug)]
pub enum PermissionsSubcommand {
    Current(ApiTarget),
    User(ResourceGetCommand),
}

#[derive(Args, Debug)]
pub struct ReportsCommand {
    #[command(subcommand)]
    pub command: ReportsSubcommand,
}

#[derive(Subcommand, Debug)]
pub enum ReportsSubcommand {
    Summary(ReportQueryCommand),
    Filter(ReportQueryCommand),
    Events(ReportQueryCommand),
    ByClient(ReportQueryCommand),
    ByProject(ReportQueryCommand),
    ByUser(ReportQueryCommand),
    ByTeam(ReportQueryCommand),
}

#[derive(Args, Debug)]
pub struct ReportQueryCommand {
    #[command(flatten)]
    pub target: ApiTarget,
    #[arg(long = "query")]
    pub query: Vec<String>,
    #[arg(long)]
    pub since: Option<String>,
    #[arg(long)]
    pub until: Option<String>,
    #[arg(long)]
    pub user_ids: Option<String>,
    #[arg(long)]
    pub project_ids: Option<String>,
    #[arg(long)]
    pub client_ids: Option<String>,
    #[arg(long)]
    pub label_ids: Option<String>,
    #[arg(long)]
    pub team_ids: Option<String>,
    #[arg(long)]
    pub state_ids: Option<String>,
    #[arg(long)]
    pub group_by: Option<String>,
    #[arg(long)]
    pub scope: Option<String>,
    #[arg(long)]
    pub billed: Option<String>,
}

#[derive(Args, Debug)]
pub struct ProjectCreateCommand {
    #[command(flatten)]
    pub target: ApiTarget,
    #[arg(long)]
    pub name: String,
    #[arg(long)]
    pub rate_type: String,
    #[arg(long)]
    pub color: Option<String>,
    #[arg(long)]
    pub description: Option<String>,
    #[arg(long)]
    pub company_id: Option<i64>,
    #[arg(long)]
    pub client_id: Option<i64>,
    #[arg(long)]
    pub new_company: Option<String>,
    #[arg(long)]
    pub hour_rate: Option<f64>,
    #[arg(long)]
    pub budget: Option<f64>,
    #[arg(long)]
    pub budget_type: Option<String>,
    #[arg(long)]
    pub billable: Option<bool>,
    #[arg(long)]
    pub active: Option<bool>,
    #[arg(long)]
    pub external_id: Option<String>,
    #[arg(long)]
    pub budget_scope: Option<String>,
    #[arg(long)]
    pub send_invite: Option<bool>,
    #[arg(long)]
    pub update_hour_billable_state: Option<bool>,
    #[arg(long)]
    pub currency_code: Option<String>,
    #[arg(long)]
    pub exchange_rate: Option<String>,
    #[arg(long = "team-id")]
    pub team_ids: Vec<i64>,
    #[arg(long = "label-id")]
    pub label_ids: Vec<i64>,
    #[arg(long = "required-label-id")]
    pub required_label_ids: Vec<i64>,
    #[arg(long = "user-rate")]
    pub user_rates: Vec<String>,
}

#[derive(Args, Debug)]
pub struct ProjectUpdateCommand {
    pub id: i64,
    #[command(flatten)]
    pub target: ApiTarget,
    #[arg(long)]
    pub name: Option<String>,
    #[arg(long)]
    pub rate_type: Option<String>,
    #[arg(long)]
    pub color: Option<String>,
    #[arg(long)]
    pub description: Option<String>,
    #[arg(long)]
    pub company_id: Option<i64>,
    #[arg(long)]
    pub client_id: Option<i64>,
    #[arg(long)]
    pub new_company: Option<String>,
    #[arg(long)]
    pub hour_rate: Option<f64>,
    #[arg(long)]
    pub budget: Option<f64>,
    #[arg(long)]
    pub budget_type: Option<String>,
    #[arg(long)]
    pub billable: Option<bool>,
    #[arg(long)]
    pub active: Option<bool>,
    #[arg(long)]
    pub external_id: Option<String>,
    #[arg(long)]
    pub budget_scope: Option<String>,
    #[arg(long)]
    pub send_invite: Option<bool>,
    #[arg(long)]
    pub update_hour_billable_state: Option<bool>,
    #[arg(long)]
    pub currency_code: Option<String>,
    #[arg(long)]
    pub exchange_rate: Option<String>,
    #[arg(long = "team-id")]
    pub team_ids: Vec<i64>,
    #[arg(long = "label-id")]
    pub label_ids: Vec<i64>,
    #[arg(long = "required-label-id")]
    pub required_label_ids: Vec<i64>,
    #[arg(long = "user-rate")]
    pub user_rates: Vec<String>,
}
