// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use marreq::app;

#[rocket::main]
#[allow(clippy::result_large_err)]
async fn main() -> Result<(), rocket::Error> {
    app::build_with(marreq::deployment::default_mode(), Vec::new(), Vec::new())
        .launch()
        .await?;

    Ok(())
}
