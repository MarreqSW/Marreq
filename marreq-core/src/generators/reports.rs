// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Collects project data for the requirements table PDF rendered in
//! [`crate::helper_functions::reports`]. Report documents (VCD, coverage) are
//! in [`crate::reports`].

use std::collections::HashMap;

use crate::generators::GeneratorError;
use crate::helper_functions::reports::{RequirementsPdfRow, generate_requirements_pdf_report};
use crate::repository::Repository;

/// Build the requirements table report (one row per requirement) as PDF bytes.
pub fn requirements_pdf_with_repo<R: Repository>(
    repo: &R,
    project_id: i32,
) -> Result<Vec<u8>, GeneratorError> {
    let project = repo
        .get_project_by_id(project_id)
        .map_err(|e| format!("Error querying project: {:?}", e))?;
    let requirements = repo
        .get_requirements_by_project(project_id)
        .map_err(|e| format!("Error querying requirements by project: {:?}", e))?;
    let custom_defs = repo
        .list_custom_field_definitions_by_project(project_id)
        .map_err(|e| format!("Error querying custom field definitions: {:?}", e))?;
    let status_titles = status_titles(repo, project_id)?;

    let rows: Vec<RequirementsPdfRow> = requirements
        .iter()
        .map(|req| {
            let values = req
                .current_version_id
                .map(|version_id| {
                    repo.get_custom_field_values_for_version(version_id)
                        .unwrap_or_default()
                })
                .unwrap_or_default();
            let by_field: HashMap<i32, String> = values
                .into_iter()
                .map(|value| (value.field_id, value.value.unwrap_or_default()))
                .collect();
            let custom_values = custom_defs
                .iter()
                .map(|def| by_field.get(&def.id).cloned().unwrap_or_default())
                .collect();
            (
                req.id,
                req.title.clone(),
                req.reference_code.clone(),
                status_titles
                    .get(&req.status_id)
                    .cloned()
                    .unwrap_or_else(|| format!("Status #{}", req.status_id)),
                custom_values,
            )
        })
        .collect();

    let headers: Vec<String> = custom_defs.iter().map(|def| def.label.clone()).collect();
    generate_requirements_pdf_report(&project.name, &rows, &headers)
        .map_err(|e| format!("{e}").into())
}

fn status_titles<R: Repository>(
    repo: &R,
    project_id: i32,
) -> Result<HashMap<i32, String>, GeneratorError> {
    Ok(repo
        .get_requirement_status_by_project(project_id)
        .map_err(|e| format!("Error querying requirement statuses: {:?}", e))?
        .into_iter()
        .map(|status| (status.id, status.title))
        .collect())
}
