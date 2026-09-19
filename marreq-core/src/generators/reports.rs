// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Collects project data for the PDF reports rendered in
//! [`crate::helper_functions::reports`].

use std::collections::{HashMap, HashSet};

use crate::generators::GeneratorError;
use crate::helper_functions::reports::{
    generate_pdf_report_data, generate_requirements_pdf_report, Metrics, RequirementsPdfRow,
};
use crate::repository::Repository;

/// Build the project summary report (totals, coverage, status breakdowns) as PDF bytes.
pub fn project_report_pdf_with_repo<R: Repository>(
    repo: &R,
    project_id: i32,
) -> Result<Vec<u8>, GeneratorError> {
    let metrics = project_metrics(repo, project_id)?;
    generate_pdf_report_data(&metrics).map_err(|e| format!("{e}").into())
}

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

fn project_metrics<R: Repository>(repo: &R, project_id: i32) -> Result<Metrics, GeneratorError> {
    let requirements = repo
        .get_requirements_by_project(project_id)
        .map_err(|e| format!("Error querying requirements by project: {:?}", e))?;
    let verifications = repo
        .get_verifications_by_project(project_id)
        .map_err(|e| format!("Error querying verifications by project: {:?}", e))?;
    let links = repo
        .get_matrix_by_project(project_id)
        .map_err(|e| format!("Error querying matrix links: {:?}", e))?;
    let categories = repo
        .get_categories_by_project(project_id)
        .map_err(|e| format!("Error querying categories: {:?}", e))?;
    let statuses = repo
        .get_requirement_status_by_project(project_id)
        .map_err(|e| format!("Error querying requirement statuses: {:?}", e))?;
    let verification_statuses = repo
        .get_verification_status_by_project(project_id)
        .map_err(|e| format!("Error querying verification statuses: {:?}", e))?;
    let members = repo
        .get_members_by_project(project_id)
        .map_err(|e| format!("Error querying project members: {:?}", e))?;

    let status_titles: HashMap<i32, String> = statuses
        .iter()
        .map(|status| (status.id, status.title.clone()))
        .collect();
    let verification_status_titles: HashMap<i32, String> = verification_statuses
        .iter()
        .map(|status| (status.id, status.title.clone()))
        .collect();
    let category_titles: HashMap<i32, String> = categories
        .iter()
        .map(|category| (category.id, category.title.clone()))
        .collect();

    let mut requirements_by_status: HashMap<String, i32> = HashMap::new();
    let mut requirements_by_category: HashMap<String, i32> = HashMap::new();
    for req in &requirements {
        let status = status_titles
            .get(&req.status_id)
            .cloned()
            .unwrap_or_else(|| format!("Status #{}", req.status_id));
        *requirements_by_status.entry(status).or_insert(0) += 1;
        let category = category_titles
            .get(&req.category_id)
            .cloned()
            .unwrap_or_else(|| format!("Category #{}", req.category_id));
        *requirements_by_category.entry(category).or_insert(0) += 1;
    }

    let mut tests_by_status: HashMap<String, i32> = HashMap::new();
    for verification in &verifications {
        let status = verification_status_titles
            .get(&verification.status_id)
            .cloned()
            .unwrap_or_else(|| format!("Status #{}", verification.status_id));
        *tests_by_status.entry(status).or_insert(0) += 1;
    }

    let covered: HashSet<i32> = links.iter().map(|link| link.req_id).collect();
    let covered_requirements = requirements
        .iter()
        .filter(|req| covered.contains(&req.id))
        .count();
    let total_requirements = requirements.len();
    let coverage_percentage = if total_requirements == 0 {
        0.0
    } else {
        (covered_requirements as f64 / total_requirements as f64) * 100.0
    };
    let avg_tests_per_requirement = if total_requirements == 0 {
        0.0
    } else {
        links.len() as f64 / total_requirements as f64
    };

    Ok(Metrics {
        categories,
        statuses,
        users_len: members.len(),
        total_requirements,
        total_tests: verifications.len(),
        total_categories: category_titles.len(),
        requirements_by_status,
        tests_by_status,
        requirements_by_category,
        covered_requirements,
        total_links: links.len(),
        coverage_percentage,
        avg_tests_per_requirement,
        // Only the HTML renderer reports recency, and verifications carry no creation date.
        recent_requirements: 0,
        recent_tests: 0,
    })
}
