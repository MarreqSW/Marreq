// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use rocket::http::Status;
use rocket::Request;

pub(crate) fn extract_route_param(
    request: &Request<'_>,
    placeholder: &str,
) -> Result<String, Status> {
    let route = request.route().ok_or(Status::InternalServerError)?;

    let route_segments: Vec<_> = route
        .uri
        .path()
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();

    let request_segments: Vec<_> = request
        .uri()
        .path()
        .segments()
        .filter(|segment| !segment.is_empty())
        .collect();

    let project_index = route_segments
        .iter()
        .position(|segment| *segment == placeholder)
        .or_else(|| {
            // Rocket route may use `<_project_id>` for an unused binding; guard still needs the segment index.
            if placeholder == "<project_id>" {
                route_segments
                    .iter()
                    .position(|segment| *segment == "<_project_id>")
            } else {
                None
            }
        })
        .ok_or(Status::InternalServerError)?;

    request_segments
        .get(project_index)
        .copied()
        .map(|segment| segment.to_string())
        .ok_or(Status::BadRequest)
}
