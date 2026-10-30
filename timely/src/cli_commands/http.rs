use clap::{Args, Subcommand, ValueEnum};

#[derive(Args, Debug)]
pub struct CallCommand {
    pub operation: String,
    #[arg(long)]
    pub account_id: Option<i64>,
    #[arg(long = "param")]
    pub params: Vec<String>,
    #[arg(long = "query")]
    pub query: Vec<String>,
    #[arg(long)]
    pub body: Option<String>,
    #[arg(long = "body-file")]
    pub body_file: Option<String>,
}

#[derive(Args, Debug)]
pub struct RequestCommand {
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

#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum HttpVerb {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

#[derive(Args, Debug)]
pub struct McpCommand {
    #[command(subcommand)]
    pub command: McpSubcommand,
}

#[derive(Subcommand, Debug)]
pub enum McpSubcommand {
    /// Serve MCP tools over stdin/stdout
    Serve,
}
