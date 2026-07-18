use clap::{Args, Subcommand};

#[derive(Args, Debug)]
pub struct MemoryCommand {
    #[command(subcommand)]
    pub command: MemorySubcommand,
}

#[derive(Subcommand, Debug)]
pub enum MemorySubcommand {
    /// Show Memory database status
    Status {
        #[arg(long, help = "Override Memory database path")]
        db_path: Option<String>,
    },
    /// List apps from Memory
    Apps {
        #[arg(long, default_value_t = 25)]
        limit: usize,
        #[arg(long, help = "Override Memory database path")]
        db_path: Option<String>,
    },
    /// List recent Memory events
    Recent {
        #[arg(long, default_value_t = 25)]
        limit: usize,
        #[arg(long)]
        app: Option<String>,
        #[arg(long)]
        include_details: bool,
        #[arg(long, help = "Override Memory database path")]
        db_path: Option<String>,
    },
    /// Search Memory events
    Search {
        query: String,
        #[arg(long, default_value_t = 25)]
        limit: usize,
        #[arg(long)]
        app: Option<String>,
        #[arg(long)]
        include_details: bool,
        #[arg(long, help = "Override Memory database path")]
        db_path: Option<String>,
    },
    /// Export Memory entries as JSON
    Export {
        #[arg(long, default_value_t = 1000)]
        limit: usize,
        #[arg(long)]
        app: Option<String>,
        #[arg(long, help = "Include entries on or after this captured_at_utc")]
        since: Option<String>,
        #[arg(long, help = "Include entries on or before this captured_at_utc")]
        upto: Option<String>,
        #[arg(long)]
        include_details: bool,
        #[arg(long, help = "Override Memory database path")]
        db_path: Option<String>,
        // `--file` avoids clashing with global `-o`/`--output` format.
        #[arg(
            long,
            short = 'f',
            value_name = "PATH",
            help = "Write JSON to a file, or - for stdout"
        )]
        file: Option<String>,
    },
}
