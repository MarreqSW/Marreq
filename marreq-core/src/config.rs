// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Centralised application configuration.
//!
//! At startup each binary calls [`AppConfig::install_from_env_or_exit`].  All
//! subsequent reads go through [`AppConfig::current`].  Sub-system specific
//! types (CORS, SMTP, semantic-search, …) live next to their consumers; this
//! module just gathers them in a single struct so misconfiguration is detected
//! once at boot instead of on the first request that needs them.

use std::sync::OnceLock;

use crate::cors::CorsPolicy;
use crate::services::semantic_search::config::SemanticSearchConfig;

/// Aggregated configuration loaded once at process startup.
pub struct AppConfig {
    /// Postgres connection string. Required.
    pub database_url: String,
    /// Public URL the application is reachable at, used in outgoing emails
    /// and OAuth callbacks. Defaults to `http://localhost:8000`.
    pub public_base_url: String,
    /// When true (default), POSTing a comment against an approved
    /// requirement_version_id returns 403.
    pub lock_approved_version_comments: bool,
    /// When true, the session cookie uses the `__Host-` prefix and `Secure`.
    /// Defaults to false so HTTP localhost works out of the box.
    pub secure_session_cookie: bool,
    /// CORS allow-list and credentials policy.
    pub cors: CorsPolicy,
    /// Origins permitted to send state-changing requests (CSRF guard).
    pub csrf_allowed_origins: Vec<String>,
    /// Semantic search / embeddings configuration.
    pub semantic: SemanticSearchConfig,
}

static CONFIG: OnceLock<AppConfig> = OnceLock::new();

#[derive(Debug)]
pub struct ConfigError {
    pub issues: Vec<String>,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "invalid application configuration:")?;
        for issue in &self.issues {
            writeln!(f, "  - {issue}")?;
        }
        Ok(())
    }
}

impl std::error::Error for ConfigError {}

impl AppConfig {
    /// Build a config from the current process environment, accumulating
    /// every problem before returning so operators see them all at once.
    pub fn from_env() -> Result<Self, ConfigError> {
        // Allow `.env` for local dev without requiring callers to do it.
        let _ = dotenvy::dotenv();

        let mut issues: Vec<String> = Vec::new();

        let database_url = match std::env::var("DATABASE_URL") {
            Ok(v) if !v.is_empty() => v,
            _ => {
                issues.push("DATABASE_URL must be set".into());
                String::new()
            }
        };

        let public_base_url = std::env::var("MARREQ_PUBLIC_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:8000".into());

        let lock_approved_version_comments = parse_bool_env("LOCK_APPROVED_VERSION_COMMENTS", true);
        let secure_session_cookie = parse_bool_env("MARREQ_SECURE_SESSION_COOKIE", false);

        let cors = CorsPolicy::from_env();

        let csrf_allowed_origins = std::env::var("CSRF_ALLOWED_ORIGINS")
            .map(|v| {
                v.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        let semantic = SemanticSearchConfig::from_env();

        if !issues.is_empty() {
            return Err(ConfigError { issues });
        }

        Ok(AppConfig {
            database_url,
            public_base_url,
            lock_approved_version_comments,
            secure_session_cookie,
            cors,
            csrf_allowed_origins,
            semantic,
        })
    }

    /// Store the configuration in the process-wide `OnceLock`. Subsequent
    /// calls are no-ops (returns the previously-installed instance unchanged).
    pub fn install(cfg: AppConfig) -> &'static AppConfig {
        let _ = CONFIG.set(cfg);
        CONFIG.get().expect("AppConfig set above")
    }

    /// Load from environment and install. On failure, prints the aggregated
    /// list of problems and exits with status 2.
    pub fn install_from_env_or_exit() -> &'static AppConfig {
        match Self::from_env() {
            Ok(cfg) => Self::install(cfg),
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(2);
            }
        }
    }

    /// The installed configuration. Panics if [`AppConfig::install`] has not
    /// been called yet — only the binary entry-points should hit this before
    /// installation, and they install first thing.
    pub fn current() -> &'static AppConfig {
        CONFIG
            .get()
            .expect("AppConfig::current() called before AppConfig::install()")
    }

    /// Non-panicking variant for code paths that may run before install
    /// (notably unit tests that don't go through `app::build_with`).
    pub fn try_current() -> Option<&'static AppConfig> {
        CONFIG.get()
    }
}

fn parse_bool_env(name: &str, default: bool) -> bool {
    std::env::var(name)
        .ok()
        .and_then(|s| match s.to_lowercase().as_str() {
            "1" | "true" | "yes" => Some(true),
            "0" | "false" | "no" => Some(false),
            _ => None,
        })
        .unwrap_or(default)
}

/// Backward-compatible accessor; prefer
/// `AppConfig::current().lock_approved_version_comments` in new code.
pub fn lock_approved_version_comments() -> bool {
    AppConfig::try_current()
        .map(|c| c.lock_approved_version_comments)
        .unwrap_or_else(|| parse_bool_env("LOCK_APPROVED_VERSION_COMMENTS", true))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_bool_env_defaults() {
        // Use a name that is extremely unlikely to be set in the env.
        assert!(parse_bool_env("MARREQ_TEST_MISSING_BOOL_XYZZY", true));
        assert!(!parse_bool_env("MARREQ_TEST_MISSING_BOOL_XYZZY", false));
    }

    #[test]
    fn config_error_formats_all_issues() {
        let err = ConfigError {
            issues: vec!["a".into(), "b".into()],
        };
        let s = format!("{err}");
        assert!(s.contains("a"));
        assert!(s.contains("b"));
    }
}
