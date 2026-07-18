use clap::{Args, Subcommand};

#[derive(Args, Debug)]
pub struct ConfigCommand {
    #[command(subcommand)]
    pub command: ConfigSubcommand,
}

#[derive(Subcommand, Debug)]
pub enum ConfigSubcommand {
    #[command(about = "Print TIMELY_HOME and config.env paths")]
    Path,

    #[command(about = "List known config keys and effective values")]
    List,

    #[command(about = "Get one config key")]
    Get {
        #[arg(help = "Friendly key (oauth.client_id) or TIMELY_* env name")]
        key: String,
    },

    #[command(about = "Set a config key in config.env")]
    Set {
        #[arg(help = "Friendly key (oauth.client_id) or TIMELY_* env name")]
        key: String,
        #[arg(help = "Value to store")]
        value: String,
    },

    #[command(about = "Remove a config key from config.env")]
    Unset {
        #[arg(help = "Friendly key (oauth.client_id) or TIMELY_* env name")]
        key: String,
    },
}
