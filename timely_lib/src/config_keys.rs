//! Allowlisted home-config keys (non-secret defaults only).

pub struct ConfigKeyDef {
    pub friendly: &'static str,
    pub env: &'static str,
}

pub const CONFIG_KEY_DEFS: &[ConfigKeyDef] = &[
    ConfigKeyDef {
        friendly: "oauth.client_id",
        env: "TIMELY_CLIENT_ID",
    },
    ConfigKeyDef {
        friendly: "oauth.redirect_uri",
        env: "TIMELY_REDIRECT_URI",
    },
    ConfigKeyDef {
        friendly: "account.id",
        env: "TIMELY_ACCOUNT_ID",
    },
    ConfigKeyDef {
        friendly: "api.base_url",
        env: "TIMELY_BASE_URL",
    },
    ConfigKeyDef {
        friendly: "http.timeout",
        env: "TIMELY_TIMEOUT",
    },
    ConfigKeyDef {
        friendly: "output",
        env: "TIMELY_OUTPUT",
    },
    ConfigKeyDef {
        friendly: "profile",
        env: "TIMELY_PROFILE",
    },
    ConfigKeyDef {
        friendly: "debug",
        env: "TIMELY_DEBUG",
    },
    ConfigKeyDef {
        friendly: "no_color",
        env: "TIMELY_NO_COLOR",
    },
    ConfigKeyDef {
        friendly: "memory.db",
        env: "TIMELY_MEMORY_DB",
    },
];

const REJECTED_SECRET_KEYS: &[&str] = &[
    "TIMELY_TOKEN",
    "TIMELY_REFRESH_TOKEN",
    "TIMELY_CLIENT_SECRET",
    "token",
    "refresh_token",
    "oauth.client_secret",
    "client_secret",
];

pub fn resolve_config_key_def(input: &str) -> Result<&'static ConfigKeyDef, String> {
    let input = input.trim();
    if is_rejected_secret_key(input) {
        return Err(
            "secrets are not stored in config.env; use auth token, oauth, or a password manager"
                .to_string(),
        );
    }
    CONFIG_KEY_DEFS
        .iter()
        .find(|def| def.friendly == input || def.env == input)
        .ok_or_else(|| format!("unknown config key: {input}"))
}

pub fn resolve_config_key(input: &str) -> Result<String, String> {
    resolve_config_key_def(input).map(|definition| definition.env.to_string())
}

pub fn friendly_config_key(input: &str) -> Result<String, String> {
    resolve_config_key_def(input).map(|definition| definition.friendly.to_string())
}

pub fn is_allowed_config_key(key: &str) -> bool {
    CONFIG_KEY_DEFS
        .iter()
        .any(|definition| definition.env == key)
}

fn is_rejected_secret_key(input: &str) -> bool {
    REJECTED_SECRET_KEYS
        .iter()
        .any(|key| key.eq_ignore_ascii_case(input))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_accepts_friendly_and_env_names() {
        assert_eq!(
            resolve_config_key("oauth.client_id").unwrap(),
            "TIMELY_CLIENT_ID"
        );
        assert_eq!(
            friendly_config_key("TIMELY_ACCOUNT_ID").unwrap(),
            "account.id"
        );
    }

    #[test]
    fn resolve_rejects_secrets_and_unknown_keys() {
        assert!(resolve_config_key("TIMELY_TOKEN").is_err());
        assert!(resolve_config_key("oauth.client_secret").is_err());
        assert!(resolve_config_key("api.key").is_err());
    }
}
