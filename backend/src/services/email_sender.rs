// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Best-effort email delivery using SMTP.
//!
//! If `SMTP_HOST` is not set, all sends are silently skipped so the rest of the
//! application works without a mail server.

use std::sync::OnceLock;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EmailError {
    #[error("SMTP transport error: {0}")]
    Transport(#[from] lettre::transport::smtp::Error),
    #[error("email build error: {0}")]
    Build(#[from] lettre::error::Error),
    #[error("invalid address: {0}")]
    Address(#[from] lettre::address::AddressError),
}

#[derive(Clone, Debug)]
struct SmtpConfig {
    host: String,
    port: u16,
    username: Option<String>,
    password: Option<String>,
    from_address: String,
}

static CONFIG: OnceLock<Option<SmtpConfig>> = OnceLock::new();

fn global_config() -> &'static Option<SmtpConfig> {
    CONFIG.get_or_init(|| {
        let host = std::env::var("SMTP_HOST").ok()?;
        Some(SmtpConfig {
            host,
            port: std::env::var("SMTP_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(587),
            username: std::env::var("SMTP_USERNAME").ok(),
            password: std::env::var("SMTP_PASSWORD").ok(),
            from_address: std::env::var("SMTP_FROM_ADDRESS")
                .unwrap_or_else(|_| "marreq@localhost".into()),
        })
    })
}

pub fn is_email_enabled() -> bool {
    global_config().is_some()
}

pub fn send_email(to: &str, subject: &str, body: &str) -> Result<(), EmailError> {
    use lettre::message::header::ContentType;
    use lettre::{Message, SmtpTransport, Transport};

    let config = match global_config() {
        Some(c) => c,
        None => return Ok(()),
    };

    let email = Message::builder()
        .from(config.from_address.parse()?)
        .to(to.parse()?)
        .subject(subject)
        .header(ContentType::TEXT_PLAIN)
        .body(body.to_string())?;

    let mut builder = SmtpTransport::starttls_relay(&config.host)?.port(config.port);

    if let (Some(user), Some(pass)) = (&config.username, &config.password) {
        builder = builder.credentials(lettre::transport::smtp::authentication::Credentials::new(
            user.clone(),
            pass.clone(),
        ));
    }

    let transport = builder.build();
    transport.send(&email)?;
    Ok(())
}
