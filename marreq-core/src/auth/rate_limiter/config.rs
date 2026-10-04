// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Which [`super::RateLimitStore`] the server uses (issue #286).

use std::time::Duration;

pub const DEFAULT_RETENTION_HOURS: u64 = 24;

/// `MARREQ_RATE_LIMIT_STORE`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RateLimitStoreKind {
    /// Shared `login_rate_limits` table (default): coordinated across
    /// replicas and kept over restarts.
    Postgres,
    /// Process-local maps: a single instance only.
    Memory,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RateLimitConfig {
    pub store: RateLimitStoreKind,
    /// `MARREQ_RATE_LIMIT_RETENTION_HOURS`: idle records (with no running
    /// lock) are removed after this long. Postgres store only.
    pub retention: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            store: RateLimitStoreKind::Postgres,
            retention: Duration::from_secs(DEFAULT_RETENTION_HOURS * 3600),
        }
    }
}

impl RateLimitConfig {
    /// Read from the environment, pushing a message to `issues` for each bad value.
    pub fn from_env(issues: &mut Vec<String>) -> Self {
        Self::from_lookup(|name| std::env::var(name).ok(), issues)
    }

    fn from_lookup(get: impl Fn(&str) -> Option<String>, issues: &mut Vec<String>) -> Self {
        let mut config = Self::default();
        if let Some(raw) = get("MARREQ_RATE_LIMIT_STORE") {
            match raw.trim().to_ascii_lowercase().as_str() {
                "" | "postgres" => {}
                "memory" => config.store = RateLimitStoreKind::Memory,
                other => issues.push(format!(
                    "MARREQ_RATE_LIMIT_STORE must be \"postgres\" or \"memory\", not {other:?}"
                )),
            }
        }
        if let Some(raw) = get("MARREQ_RATE_LIMIT_RETENTION_HOURS") {
            match raw.trim().parse::<u64>() {
                Ok(hours) if (1..=24 * 365).contains(&hours) => {
                    config.retention = Duration::from_secs(hours * 3600);
                }
                _ => issues.push(format!(
                    "MARREQ_RATE_LIMIT_RETENTION_HOURS must be a whole number of hours from 1 to 8760, not {raw:?}"
                )),
            }
        }
        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(vars: &[(&str, &str)]) -> (RateLimitConfig, Vec<String>) {
        let mut issues = Vec::new();
        let config = RateLimitConfig::from_lookup(
            |name| {
                vars.iter()
                    .find(|(k, _)| *k == name)
                    .map(|(_, v)| v.to_string())
            },
            &mut issues,
        );
        (config, issues)
    }

    #[test]
    fn defaults_to_the_shared_postgres_store() {
        let (config, issues) = parse(&[]);
        assert_eq!(config, RateLimitConfig::default());
        assert_eq!(config.store, RateLimitStoreKind::Postgres);
        assert_eq!(config.retention, Duration::from_secs(24 * 3600));
        assert!(issues.is_empty());
    }

    #[test]
    fn reads_store_and_retention() {
        let (config, issues) = parse(&[
            ("MARREQ_RATE_LIMIT_STORE", " Memory "),
            ("MARREQ_RATE_LIMIT_RETENTION_HOURS", "48"),
        ]);
        assert!(issues.is_empty());
        assert_eq!(config.store, RateLimitStoreKind::Memory);
        assert_eq!(config.retention, Duration::from_secs(48 * 3600));
        assert_eq!(
            parse(&[("MARREQ_RATE_LIMIT_STORE", "postgres")]).0.store,
            RateLimitStoreKind::Postgres
        );
    }

    #[test]
    fn rejects_bad_values() {
        let (_, issues) = parse(&[
            ("MARREQ_RATE_LIMIT_STORE", "redis"),
            ("MARREQ_RATE_LIMIT_RETENTION_HOURS", "0"),
        ]);
        assert_eq!(issues.len(), 2);
        assert!(issues[0].contains("MARREQ_RATE_LIMIT_STORE"));
        assert!(issues[1].contains("MARREQ_RATE_LIMIT_RETENTION_HOURS"));
        assert_eq!(
            parse(&[("MARREQ_RATE_LIMIT_RETENTION_HOURS", "a day")])
                .1
                .len(),
            1
        );
    }
}
