//! Typed errors for CLI and API boundaries.

use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum TimelyError {
    #[error("Authentication failed: {0}")]
    Auth(String),

    #[error("API error: {0}")]
    Api(String),

    #[error("{0}")]
    Usage(String),

    #[error("{0}")]
    Io(String),

    #[error("{0}")]
    Other(String),
}

impl TimelyError {
    pub fn from_anyhow(error: anyhow::Error) -> Self {
        for cause in error.chain() {
            if let Some(timely) = cause.downcast_ref::<TimelyError>() {
                return timely.clone();
            }
        }
        classify_message(&error.to_string())
    }
}

pub fn classify_message(message: &str) -> TimelyError {
    let lower = message.to_lowercase();
    if lower.contains("no token configured")
        || lower.contains("authentication failed")
        || lower.contains("account_id required")
        || lower.contains("authenticate first")
    {
        return TimelyError::Auth(message.to_string());
    }
    if lower.contains("timely api returned")
        || lower.contains("oauth token refresh failed")
        || lower.contains("oauth token exchange failed")
        || lower.contains("response exceeded")
    {
        return TimelyError::Api(message.to_string());
    }
    if lower.contains("unknown operationid")
        || lower.contains("expected key=value")
        || lower.contains("missing required path parameter")
        || lower.contains("batch ")
        || lower.contains("nested batch")
        || lower.contains("cannot run inside batch")
        || lower.contains("pass --yes")
        || lower.contains("provide --token")
        || lower.contains("secret value is empty")
        || lower.contains("use an inline secret")
        || lower.contains("provide a secret value")
        || lower.contains("refusing to prompt")
        || lower.contains("cancelled")
        || lower.contains("api path must be relative")
    {
        return TimelyError::Usage(message.to_string());
    }
    if lower.contains("read ")
        || lower.contains("write ")
        || lower.contains("failed to read")
        || lower.contains("failed to store")
        || lower.contains("failed to open")
    {
        return TimelyError::Io(message.to_string());
    }
    TimelyError::Other(message.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;

    #[test]
    fn from_anyhow_prefers_typed_source_error() {
        let error = anyhow!(TimelyError::Auth("no token configured".to_string()));
        assert!(matches!(
            TimelyError::from_anyhow(error),
            TimelyError::Auth(_)
        ));
    }

    #[test]
    fn from_anyhow_falls_back_to_message_classification() {
        let error = anyhow!("Timely API returned 500: boom");
        assert!(matches!(
            TimelyError::from_anyhow(error),
            TimelyError::Api(_)
        ));
    }
}
