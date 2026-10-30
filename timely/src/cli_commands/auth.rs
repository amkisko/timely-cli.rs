use clap::{Args, Subcommand};

use super::DEFAULT_OAUTH_REDIRECT_URI;

#[derive(Args, Debug)]
pub struct AuthCommand {
    #[command(subcommand)]
    pub command: AuthSubcommand,
}

#[derive(Subcommand, Debug)]
pub enum AuthSubcommand {
    /// Store a bearer token (default: OS keychain)
    Token(AuthTokenArgs),
    /// Show whether credentials are configured
    Status,
    /// Export redacted local auth, env, and process-cache state
    Export {
        #[arg(
            long,
            short = 'f',
            value_name = "PATH",
            help = "Write JSON to a file, or - for stdout"
        )]
        file: Option<String>,
    },
    /// Remove stored credentials for the profile
    Logout,
    /// Import a token from a password manager
    Source(AuthSourceArgs),
    /// Write a token to a password manager
    Sink(AuthSinkArgs),
    /// Run a command with TIMELY_TOKEN from stored credentials
    RunWith(AuthRunWithArgs),
    /// Authorize with Timely OAuth
    Oauth(OauthCommand),
}

#[derive(Args, Debug)]
pub struct AuthTokenArgs {
    #[arg(
        long,
        env = "TIMELY_TOKEN",
        conflicts_with = "token_file",
        help = "Bearer token (prefer --token-file or auth source)"
    )]
    pub token: Option<String>,
    #[arg(
        long = "token-file",
        value_name = "PATH",
        conflicts_with = "token",
        help = "Read token from file, or - for stdin"
    )]
    pub token_file: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Store in plain credential file (opt-in; default is OS keychain)"
    )]
    pub use_file: bool,
}

#[derive(Args, Debug)]
pub struct AuthSourceArgs {
    #[arg(long, default_value_t = true, help = "Store fetched token locally")]
    pub store: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Store in plain credential file (opt-in; default is OS keychain)"
    )]
    pub use_file: bool,
    #[command(subcommand)]
    pub provider: SecretProviderCommand,
}

#[derive(Args, Debug)]
pub struct AuthSinkArgs {
    #[arg(
        long,
        env = "TIMELY_TOKEN",
        conflicts_with_all = ["token_file", "from_profile"],
        hide_env_values = true
    )]
    pub token: Option<String>,
    #[arg(
        long = "token-file",
        value_name = "PATH",
        conflicts_with_all = ["token", "from_profile"]
    )]
    pub token_file: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["token", "token_file"],
        help = "Read access token from stored profile credentials"
    )]
    pub from_profile: bool,
    #[command(subcommand)]
    pub provider: SecretProviderCommand,
}

#[derive(Args, Debug)]
pub struct AuthRunWithArgs {
    #[arg(trailing_var_arg = true, allow_hyphen_values = true, required = true)]
    pub command: Vec<String>,
}

#[derive(Subcommand, Debug)]
pub enum SecretProviderCommand {
    Onepassword(OnepasswordSecretArgs),
    Bitwarden(BitwardenSecretArgs),
    Keepass(KeepassSecretArgs),
}

#[derive(Args, Debug)]
pub struct OnepasswordSecretArgs {
    #[arg(long, env = "OP_ACCOUNT", help = "1Password account shorthand")]
    pub account: Option<String>,
    #[arg(long)]
    pub reference: Option<String>,
    #[arg(long)]
    pub item: Option<String>,
    #[arg(long)]
    pub field: Option<String>,
    #[arg(long)]
    pub vault: Option<String>,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long, default_value_t = false)]
    pub create: bool,
}

#[derive(Args, Debug)]
pub struct BitwardenSecretArgs {
    #[arg(long)]
    pub item: Option<String>,
    #[arg(long)]
    pub field: Option<String>,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long, default_value_t = false)]
    pub create: bool,
}

#[derive(Args, Debug)]
pub struct KeepassSecretArgs {
    #[arg(long)]
    pub database: Option<String>,
    #[arg(long)]
    pub item: Option<String>,
    #[arg(long)]
    pub field: Option<String>,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long, default_value_t = false)]
    pub create: bool,
}

#[derive(Args, Debug)]
pub struct OauthCommand {
    #[arg(long, env = "TIMELY_CLIENT_ID", help = "OAuth client ID")]
    pub client_id: String,
    #[arg(
        long,
        env = "TIMELY_CLIENT_SECRET",
        conflicts_with = "client_secret_file",
        help = "OAuth client secret (prefer --client-secret-file)"
    )]
    pub client_secret: Option<String>,
    #[arg(
        long = "client-secret-file",
        value_name = "PATH",
        conflicts_with = "client_secret",
        help = "Read client secret from file, or - for stdin"
    )]
    pub client_secret_file: Option<String>,
    #[arg(long, default_value = "manage", help = "OAuth scope")]
    pub scope: String,
    #[arg(long, env = "TIMELY_REDIRECT_URI", default_value = DEFAULT_OAUTH_REDIRECT_URI)]
    pub redirect_uri: String,
    #[arg(long, help = "Open the authorize URL in a browser")]
    pub open: bool,
    #[arg(long, hide = true)]
    pub no_open: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Store OAuth in plain credential file (opt-in; default is OS keychain)"
    )]
    pub use_file: bool,
}

impl OauthCommand {
    pub fn should_open_browser(&self) -> bool {
        self.open && !self.no_open
    }

    pub fn resolved_client_secret(&self) -> anyhow::Result<Option<String>> {
        match (&self.client_secret, &self.client_secret_file) {
            (None, None) => Ok(None),
            (secret, file) => Ok(Some(timely_lib::util::read_secret_value(
                secret.clone(),
                file.clone(),
            )?)),
        }
    }
}
