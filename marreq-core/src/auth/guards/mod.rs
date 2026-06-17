// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Request guards for session, API, project, HTML project, and group access.
//!
//! Public guard types are re-exported from this module to keep existing
//! `crate::auth::guards::*` call sites stable while each guard implementation
//! lives in a focused submodule.

mod admin;
mod api;
mod bearer;
mod group;
mod html;
mod project;
mod route_params;
mod session;

pub use admin::AdminOnly;
pub use api::ApiUser;
pub use bearer::ApiUserOrBearer;
pub use group::{HtmlGroupAccess, HtmlGroupManageAccess};
pub use html::HtmlProjectAccess;
pub use project::{ProjectAccess, ProjectAccessOrBearer};
pub use session::SessionUser;
