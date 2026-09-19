// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

pub mod excel;
pub mod reports;

/// Error returned by document generators; `Send + Sync` so Rocket handlers can surface it.
pub type GeneratorError = Box<dyn std::error::Error + Send + Sync>;

#[cfg(test)]
mod tests;
