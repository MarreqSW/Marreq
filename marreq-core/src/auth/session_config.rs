// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Session lifetimes (issue #285).
//!
//! A session ends at whichever comes first:
//!
//! * the **absolute** limit, counted from sign-in
//!   (`MARREQ_SESSION_ABSOLUTE_HOURS`, default 30 days); and
//! * the **idle** limit, counted from the last activity
//!   (`MARREQ_SESSION_IDLE_MINUTES`, default 8 hours; `0` turns it off).
//!
//! Both are enforced at lookup from `created_at` and `last_seen_at`, so lowering
//! a limit also applies to sessions that already exist. Activity refreshes
//! `last_seen_at` at most once per [`SessionConfig::touch_interval`].

use std::time::Duration;

use crate::repository::SessionCutoffs;

pub const DEFAULT_ABSOLUTE_HOURS: u64 = 30 * 24;
pub const DEFAULT_IDLE_MINUTES: u64 = 8 * 60;
const MAX_ABSOLUTE_HOURS: u64 = 24 * 365;
const MIN_IDLE_MINUTES: u64 = 5;
/// Upper bound of the `last_seen_at` write throttle.
const MAX_TOUCH_INTERVAL: Duration = Duration::from_secs(60);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionConfig {
    /// Longest a session lives after sign-in.
    pub absolute: Duration,
    /// Longest a session may go without activity; `None` = no idle limit.
    pub idle: Option<Duration>,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            absolute: Duration::from_secs(DEFAULT_ABSOLUTE_HOURS * 3600),
            idle: Some(Duration::from_secs(DEFAULT_IDLE_MINUTES * 60)),
        }
    }
}

fn to_chrono(d: Duration) -> chrono::Duration {
    chrono::Duration::from_std(d).unwrap_or(chrono::Duration::MAX)
}

impl SessionConfig {
    /// The installed configuration, or the defaults (unit tests, tools).
    pub fn current() -> Self {
        crate::config::AppConfig::try_current()
            .map(|c| c.session.clone())
            .unwrap_or_default()
    }

    /// Read from the environment, pushing a message to `issues` for each bad value.
    pub fn from_env(issues: &mut Vec<String>) -> Self {
        Self::from_lookup(|name| std::env::var(name).ok(), issues)
    }

    fn from_lookup(get: impl Fn(&str) -> Option<String>, issues: &mut Vec<String>) -> Self {
        let mut config = Self::default();
        if let Some(raw) = get("MARREQ_SESSION_ABSOLUTE_HOURS") {
            match raw.trim().parse::<u64>() {
                Ok(hours) if (1..=MAX_ABSOLUTE_HOURS).contains(&hours) => {
                    config.absolute = Duration::from_secs(hours * 3600);
                }
                _ => issues.push(format!(
                    "MARREQ_SESSION_ABSOLUTE_HOURS must be a whole number of hours from 1 to {MAX_ABSOLUTE_HOURS}, not {raw:?}"
                )),
            }
        }
        if let Some(raw) = get("MARREQ_SESSION_IDLE_MINUTES") {
            match raw.trim().parse::<u64>() {
                Ok(0) => config.idle = None,
                Ok(minutes) if minutes >= MIN_IDLE_MINUTES => {
                    config.idle = Some(Duration::from_secs(minutes * 60));
                }
                _ => issues.push(format!(
                    "MARREQ_SESSION_IDLE_MINUTES must be 0 (off) or a whole number of minutes from {MIN_IDLE_MINUTES}, not {raw:?}"
                )),
            }
        }
        if let Some(idle) = config.idle
            && idle > config.absolute
        {
            issues.push(
                "MARREQ_SESSION_IDLE_MINUTES must not be longer than MARREQ_SESSION_ABSOLUTE_HOURS"
                    .into(),
            );
        }
        config
    }

    /// `expires_at` of a session created at `now`.
    pub fn expires_at(&self, now: chrono::NaiveDateTime) -> chrono::NaiveDateTime {
        now + to_chrono(self.absolute)
    }

    /// The limits as points in time, for the repository.
    pub fn cutoffs(&self, now: chrono::NaiveDateTime) -> SessionCutoffs {
        SessionCutoffs {
            now,
            created_after: now - to_chrono(self.absolute),
            seen_after: self.idle.map(|idle| now - to_chrono(idle)),
        }
    }

    /// `last_seen_at` is refreshed only when older than this: a minute, or a
    /// tenth of a shorter idle limit, so it is accurate enough without a write
    /// on every request.
    pub fn touch_interval(&self) -> Duration {
        match self.idle {
            Some(idle) => (idle / 10).min(MAX_TOUCH_INTERVAL),
            None => MAX_TOUCH_INTERVAL,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(vars: &[(&str, &str)]) -> (SessionConfig, Vec<String>) {
        let mut issues = Vec::new();
        let config = SessionConfig::from_lookup(
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
    fn defaults_are_30_days_and_8_hours() {
        let (config, issues) = parse(&[]);
        assert!(issues.is_empty());
        assert_eq!(config.absolute, Duration::from_secs(30 * 24 * 3600));
        assert_eq!(config.idle, Some(Duration::from_secs(8 * 3600)));
    }

    #[test]
    fn reads_values_and_zero_turns_idle_off() {
        let (config, issues) = parse(&[
            ("MARREQ_SESSION_ABSOLUTE_HOURS", "12"),
            ("MARREQ_SESSION_IDLE_MINUTES", "30"),
        ]);
        assert!(issues.is_empty());
        assert_eq!(config.absolute, Duration::from_secs(12 * 3600));
        assert_eq!(config.idle, Some(Duration::from_secs(30 * 60)));
        assert_eq!(parse(&[("MARREQ_SESSION_IDLE_MINUTES", "0")]).0.idle, None);
    }

    #[test]
    fn rejects_bad_values() {
        for (name, value) in [
            ("MARREQ_SESSION_ABSOLUTE_HOURS", "0"),
            ("MARREQ_SESSION_ABSOLUTE_HOURS", "9000"),
            ("MARREQ_SESSION_ABSOLUTE_HOURS", "a month"),
            ("MARREQ_SESSION_IDLE_MINUTES", "2"),
            ("MARREQ_SESSION_IDLE_MINUTES", "-1"),
        ] {
            let (_, issues) = parse(&[(name, value)]);
            assert_eq!(issues.len(), 1, "{name}={value}");
            assert!(issues[0].contains(name));
        }
        // The idle limit cannot outlast the absolute one.
        let (_, issues) = parse(&[
            ("MARREQ_SESSION_ABSOLUTE_HOURS", "1"),
            ("MARREQ_SESSION_IDLE_MINUTES", "120"),
        ]);
        assert_eq!(issues.len(), 1);
    }

    #[test]
    fn touch_interval_is_a_minute_or_a_tenth_of_a_short_idle_limit() {
        assert_eq!(
            SessionConfig::default().touch_interval(),
            Duration::from_secs(60)
        );
        let short = SessionConfig {
            idle: Some(Duration::from_secs(5 * 60)),
            ..SessionConfig::default()
        };
        assert_eq!(short.touch_interval(), Duration::from_secs(30));
        let off = SessionConfig {
            idle: None,
            ..SessionConfig::default()
        };
        assert_eq!(off.touch_interval(), Duration::from_secs(60));
    }

    #[test]
    fn cutoffs_follow_the_limits() {
        let now = chrono::NaiveDate::from_ymd_opt(2026, 10, 4)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        let config = SessionConfig::default();
        let cutoffs = config.cutoffs(now);
        assert_eq!(cutoffs.now, now);
        assert_eq!(cutoffs.created_after, now - chrono::Duration::days(30));
        assert_eq!(cutoffs.seen_after, Some(now - chrono::Duration::hours(8)));
        assert_eq!(config.expires_at(now), now + chrono::Duration::days(30));
        let off = SessionConfig {
            idle: None,
            ..config
        };
        assert_eq!(off.cutoffs(now).seen_after, None);
    }

    #[test]
    fn cutoffs_allow_only_sessions_within_every_limit() {
        let now = chrono::NaiveDate::from_ymd_opt(2026, 10, 4)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        let session =
            |created_h: i64, seen_h: i64, expires_h: i64| crate::models::entities::Session {
                token_hash: "x".into(),
                user_id: 1,
                created_at: now - chrono::Duration::hours(created_h),
                last_seen_at: now - chrono::Duration::hours(seen_h),
                expires_at: now + chrono::Duration::hours(expires_h),
                user_agent: None,
                ip_addr: None,
            };
        let on = SessionConfig::default().cutoffs(now);
        assert!(on.allows(&session(24, 1, 100)), "active");
        assert!(!on.allows(&session(24, 9, 100)), "idle for 9 h");
        assert!(!on.allows(&session(31 * 24, 1, 100)), "past 30 days");
        assert!(!on.allows(&session(24, 1, -1)), "past expires_at");
        let off = SessionConfig {
            idle: None,
            ..SessionConfig::default()
        }
        .cutoffs(now);
        assert!(off.allows(&session(24, 9, 100)), "no idle limit");
    }
}
