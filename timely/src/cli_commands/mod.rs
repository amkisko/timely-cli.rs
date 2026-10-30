use clap::{Args, Subcommand};

pub mod auth;
pub mod http;

pub use auth::*;
pub use http::*;

pub const DEFAULT_PROFILE: &str = "default";
pub const DEFAULT_BASE_URL: &str = "https://api.timelyapp.com";
pub const DEFAULT_OAUTH_REDIRECT_URI: &str = "urn:ietf:wg:oauth:2.0:oob";

#[derive(Args, Debug)]
pub struct SpecCommand {
    #[command(subcommand)]
    pub command: SpecSubcommand,
}

#[derive(Subcommand, Debug)]
pub enum SpecSubcommand {
    /// Summarize the vendored OpenAPI document
    Summary,
    /// List OpenAPI operations
    Operations {
        #[arg(long, help = "Filter operations by tag")]
        tag: Option<String>,
    },
    /// List OpenAPI schemas
    Schemas,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Cli, Commands};
    use clap::Parser;

    #[test]
    fn oauth_defaults_to_out_of_band_redirect_uri() {
        let oauth = parse_oauth_command(&["timely", "auth", "oauth", "--client-id", "client-id"]);
        assert_eq!(oauth.redirect_uri, super::DEFAULT_OAUTH_REDIRECT_URI);
    }

    #[test]
    fn oauth_does_not_open_browser_by_default() {
        let oauth = parse_oauth_command(&[
            "timely",
            "auth",
            "oauth",
            "--client-id",
            "client-id",
            "--redirect-uri",
            DEFAULT_OAUTH_REDIRECT_URI,
        ]);
        assert!(!oauth.should_open_browser());
    }

    #[test]
    fn oauth_open_flag_enables_browser_launch() {
        let oauth = parse_oauth_command(&[
            "timely",
            "auth",
            "oauth",
            "--client-id",
            "client-id",
            "--redirect-uri",
            DEFAULT_OAUTH_REDIRECT_URI,
            "--open",
        ]);
        assert!(oauth.should_open_browser());
    }

    fn parse_oauth_command(arguments: &[&str]) -> OauthCommand {
        let cli = Cli::try_parse_from(arguments).unwrap();
        let Commands::Auth(AuthCommand {
            command: AuthSubcommand::Oauth(oauth),
        }) = cli.command
        else {
            panic!("expected oauth command");
        };
        oauth
    }
}
