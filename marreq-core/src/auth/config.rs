// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use super::{ProviderDiscovery, PublicAuthProvider};
use crate::models::UserIdentity;
use std::collections::HashSet;
use url::Url;

#[derive(Clone)]
pub struct AuthProviderConfig {
    pub key: String,
    pub display_name: String,
    pub kind: ProviderKind,
    pub client_id: String,
    pub(crate) client_secret: String,
    pub auto_register: bool,
}

#[derive(Clone)]
pub enum ProviderKind {
    Oidc {
        issuer: String,
    },
    OAuth2 {
        issuer: String,
        authorization_url: String,
        token_url: String,
        userinfo_url: String,
    },
}

impl AuthProviderConfig {
    pub fn client_secret(&self) -> &str {
        &self.client_secret
    }
}

#[derive(Clone)]
pub struct AuthConfig {
    pub password_enabled: bool,
    providers: Vec<AuthProviderConfig>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct UsableAuthenticationMethods {
    password: bool,
    external_identity_ids: HashSet<i32>,
}

impl UsableAuthenticationMethods {
    pub fn allows_unlinking(&self, identity_id: i32) -> bool {
        self.password
            || self
                .external_identity_ids
                .iter()
                .any(|candidate| *candidate != identity_id)
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self::new(true, Vec::new())
    }
}

impl AuthConfig {
    pub fn new(password_enabled: bool, providers: Vec<AuthProviderConfig>) -> Self {
        Self {
            password_enabled,
            providers,
        }
    }

    pub fn from_env(deployment: &str) -> Result<Self, String> {
        let password_enabled = env_bool("MARREQ_PASSWORD_AUTH_ENABLED", true);
        let providers = if deployment == "cloud" {
            cloud_providers()?
        } else {
            server_providers()?
        };
        if !password_enabled && providers.is_empty() {
            return Err("at least one authentication method must be enabled".into());
        }
        Ok(Self {
            password_enabled,
            providers,
        })
    }

    pub fn server_from_env() -> Result<Self, String> {
        Self::from_env("server")
    }
    pub fn cloud_from_env() -> Result<Self, String> {
        Self::from_env("cloud")
    }

    pub fn provider(&self, key: &str) -> Option<&AuthProviderConfig> {
        self.providers.iter().find(|provider| provider.key == key)
    }

    pub fn usable_authentication_methods(
        &self,
        password_configured: bool,
        identities: &[UserIdentity],
    ) -> UsableAuthenticationMethods {
        let external_identity_ids = identities
            .iter()
            .filter(|identity| self.provider(&identity.provider_key).is_some())
            .map(|identity| identity.id)
            .collect();
        UsableAuthenticationMethods {
            password: self.password_enabled && password_configured,
            external_identity_ids,
        }
    }

    pub fn discovery(&self) -> ProviderDiscovery {
        ProviderDiscovery {
            password_enabled: self.password_enabled,
            external: self
                .providers
                .iter()
                .map(|provider| PublicAuthProvider {
                    id: provider.key.clone(),
                    display_name: provider.display_name.clone(),
                })
                .collect(),
        }
    }
}

fn server_providers() -> Result<Vec<AuthProviderConfig>, String> {
    if !env_bool("MARREQ_OIDC_ENABLED", false) {
        return Ok(Vec::new());
    }
    let issuer = required("MARREQ_OIDC_ISSUER")?;
    validate_endpoint(&issuer, "MARREQ_OIDC_ISSUER")?;
    Ok(vec![AuthProviderConfig {
        key: "oidc".into(),
        display_name: std::env::var("MARREQ_OIDC_DISPLAY_NAME")
            .unwrap_or_else(|_| "Company SSO".into()),
        kind: ProviderKind::Oidc { issuer },
        client_id: required("MARREQ_OIDC_CLIENT_ID")?,
        client_secret: required("MARREQ_OIDC_CLIENT_SECRET")?,
        auto_register: env_bool("MARREQ_OIDC_AUTO_REGISTER", false),
    }])
}

fn cloud_providers() -> Result<Vec<AuthProviderConfig>, String> {
    let mut providers = Vec::new();
    if let Some(provider) = optional_oidc("google", "Google", "https://accounts.google.com", true)?
    {
        providers.push(provider);
    }
    if let Some(provider) = optional_oauth(
        "github",
        "GitHub",
        "https://github.com",
        "https://github.com/login/oauth/authorize",
        "https://github.com/login/oauth/access_token",
        "https://api.github.com/user",
    )? {
        providers.push(provider);
    }
    if let Some(provider) = optional_oauth(
        "gitlab",
        "GitLab",
        "https://gitlab.com",
        "https://gitlab.com/oauth/authorize",
        "https://gitlab.com/oauth/token",
        "https://gitlab.com/api/v4/user",
    )? {
        providers.push(provider);
    }
    Ok(providers)
}

fn optional_oidc(
    key: &str,
    display: &str,
    issuer: &str,
    auto_register: bool,
) -> Result<Option<AuthProviderConfig>, String> {
    let prefix = key.to_ascii_uppercase();
    let client_id = std::env::var(format!("MARREQ_{prefix}_CLIENT_ID")).ok();
    let secret = std::env::var(format!("MARREQ_{prefix}_CLIENT_SECRET")).ok();
    match (client_id, secret) {
        (None, None) => Ok(None),
        (Some(client_id), Some(client_secret)) if !client_id.is_empty() && !client_secret.is_empty() => Ok(Some(AuthProviderConfig { key: key.into(), display_name: display.into(), kind: ProviderKind::Oidc { issuer: issuer.into() }, client_id, client_secret, auto_register })),
        _ => Err(format!("MARREQ_{prefix}_CLIENT_ID and MARREQ_{prefix}_CLIENT_SECRET must be configured together")),
    }
}

fn optional_oauth(
    key: &str,
    display: &str,
    issuer: &str,
    authorization_url: &str,
    token_url: &str,
    userinfo_url: &str,
) -> Result<Option<AuthProviderConfig>, String> {
    let Some(mut provider) = optional_oidc(key, display, issuer, true)? else {
        return Ok(None);
    };
    provider.kind = ProviderKind::OAuth2 {
        issuer: issuer.into(),
        authorization_url: authorization_url.into(),
        token_url: token_url.into(),
        userinfo_url: userinfo_url.into(),
    };
    Ok(Some(provider))
}

fn validate_endpoint(value: &str, name: &str) -> Result<(), String> {
    let url = Url::parse(value).map_err(|_| format!("{name} must be a valid URL"))?;
    let local = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "::1"));
    if url.scheme() != "https" && !(url.scheme() == "http" && local) {
        return Err(format!(
            "{name} must use HTTPS (HTTP is allowed only for localhost)"
        ));
    }
    Ok(())
}

fn required(name: &str) -> Result<String, String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{name} must be set"))
}
fn env_bool(name: &str, default: bool) -> bool {
    std::env::var(name)
        .ok()
        .and_then(|value| match value.to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" => Some(true),
            "0" | "false" | "no" => Some(false),
            _ => None,
        })
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn endpoint_requires_https_except_localhost() {
        assert!(validate_endpoint("https://sso.example.com", "issuer").is_ok());
        assert!(validate_endpoint("http://localhost:8080", "issuer").is_ok());
        assert!(validate_endpoint("http://sso.example.com", "issuer").is_err());
    }
}
