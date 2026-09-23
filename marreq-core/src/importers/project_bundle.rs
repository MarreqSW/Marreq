// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Versioned JSON project snapshot (`marreq.project-bundle.v1`).
//! Portable via tags, reference codes, and usernames — not numeric database ids.

#![allow(clippy::too_many_arguments)]

use std::collections::{HashMap, HashSet};

use rocket::serde::{Deserialize, Serialize};

use crate::app::{AppState, DieselCachedRepo};
use crate::models::{
    CustomFieldDefinitionPayload, CustomFieldValueInput, NewApplicability, NewCategory, NewProject,
    NewProjectMember, NewRequirement, NewRequirementComment, NewRequirementStatus, NewVerification,
    NewVerificationMethod, NewVerificationStatus, UpdateProject, User,
};
use crate::namespaces::project_base_path;
use crate::permissions::{ROLE_ADMIN, ROLE_AUTHOR, ROLE_REVIEWER, ROLE_VIEWER};
use crate::repository::errors::RepoError;
use crate::repository::{
    CustomFieldRepository, LookupRepository, MatrixRepository, ProjectMembersRepository,
    ProjectReviewersRepository, ProjectsRepository, RequirementCommentsRepository,
    RequirementVersionLinksRepository, RequirementsRepository, UserRepository,
    VerificationsRepository,
};
use crate::services::applicability_service::ApplicabilityService;
use crate::services::category_service::CategoryService;
use crate::services::custom_field_service::CustomFieldService;
use crate::services::matrix_service::MatrixService;
use crate::services::project_service::ProjectService;
use crate::services::requirement_service::RequirementService;
use crate::services::status_service::StatusService;
use crate::services::verification_service::VerificationService;
use crate::status_enums::ProjectStatus;

pub const FORMAT_V1: &str = "marreq.project-bundle.v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct ProjectBundle {
    pub format: String,
    pub project: BundleProject,
    #[serde(default)]
    pub catalog: BundleCatalog,
    #[serde(default)]
    pub members: Vec<BundleMember>,
    #[serde(default)]
    pub reviewers: Vec<String>,
    #[serde(default)]
    pub requirements: Vec<BundleRequirement>,
    #[serde(default)]
    pub verifications: Vec<BundleVerification>,
    #[serde(default)]
    pub matrix: Vec<BundleMatrixLink>,
    #[serde(default)]
    pub comments: Vec<BundleComment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct BundleProject {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct BundleCatalog {
    #[serde(default)]
    pub categories: Vec<BundleTagged>,
    #[serde(default)]
    pub applicability: Vec<BundleTagged>,
    #[serde(default)]
    pub requirement_statuses: Vec<BundleStatus>,
    #[serde(default)]
    pub verification_statuses: Vec<BundleStatus>,
    #[serde(default)]
    pub verification_methods: Vec<BundleTagged>,
    #[serde(default)]
    pub custom_fields: Vec<BundleCustomField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct BundleTagged {
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub tag: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct BundleStatus {
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub tag: String,
    #[serde(default)]
    pub tag_color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct BundleCustomField {
    pub label: String,
    pub field_type: String,
    #[serde(default)]
    pub enum_values: Option<Vec<String>>,
    #[serde(default)]
    pub sort_order: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct BundleMember {
    pub username: String,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct BundleRequirement {
    pub reference_code: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub justification: Option<String>,
    #[serde(default)]
    pub status_tag: Option<String>,
    #[serde(default)]
    pub category_tag: Option<String>,
    #[serde(default)]
    pub applicability_tag: Option<String>,
    #[serde(default)]
    pub author_username: Option<String>,
    #[serde(default)]
    pub reviewer_username: Option<String>,
    #[serde(default)]
    pub verification_method_tags: Vec<String>,
    #[serde(default)]
    pub custom_fields: Vec<BundleCustomValue>,
    #[serde(default)]
    pub parent_links: Vec<BundleParentLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct BundleCustomValue {
    pub label: String,
    #[serde(default)]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct BundleParentLink {
    pub reference_code: String,
    #[serde(default)]
    pub link_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct BundleVerification {
    pub reference_code: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub status_tag: Option<String>,
    #[serde(default)]
    pub verification_method_tag: Option<String>,
    #[serde(default)]
    pub parent_reference_code: Option<String>,
    #[serde(default)]
    pub author_username: Option<String>,
    #[serde(default)]
    pub reviewer_username: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct BundleMatrixLink {
    pub requirement_reference_code: String,
    pub verification_reference_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct BundleComment {
    pub requirement_reference_code: String,
    pub body: String,
    #[serde(default)]
    pub author_username: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct BundleImportedCounts {
    pub categories: usize,
    pub applicability: usize,
    pub requirement_statuses: usize,
    pub verification_statuses: usize,
    pub verification_methods: usize,
    pub custom_fields: usize,
    pub requirements: usize,
    pub verifications: usize,
    pub matrix_links: usize,
    pub comments: usize,
    pub members: usize,
    pub reviewers: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct BundleImportResult {
    pub project_id: i32,
    pub slug: String,
    pub project_base_path: String,
    pub imported_counts: BundleImportedCounts,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

pub fn parse_bundle(bytes: &[u8]) -> Result<ProjectBundle, String> {
    let bundle: ProjectBundle =
        serde_json::from_slice(bytes).map_err(|e| format!("invalid project bundle JSON: {e}"))?;
    if bundle.format != FORMAT_V1 {
        return Err(format!(
            "unsupported bundle format {:?}; expected {}",
            bundle.format, FORMAT_V1
        ));
    }
    if bundle.project.name.trim().is_empty() {
        return Err("bundle project.name is required".into());
    }
    Ok(bundle)
}

pub fn export_bundle(
    state: &AppState<DieselCachedRepo>,
    project_id: i32,
) -> Result<ProjectBundle, RepoError> {
    let repo = state.repo_read();
    let project = repo.get_project_by_id(project_id)?;
    let users: HashMap<i32, User> = repo
        .get_users_all()?
        .into_iter()
        .map(|u| (u.id, u))
        .collect();
    let username = |id: i32| users.get(&id).map(|u| u.username.clone());

    let categories = repo.get_categories_by_project(project_id)?;
    let applicability = repo.get_applicability_by_project(project_id)?;
    let req_statuses = repo.get_requirement_status_by_project(project_id)?;
    let ver_statuses = repo.get_verification_status_by_project(project_id)?;
    let methods = repo.get_verification_methods_by_project(project_id)?;
    let custom_defs = repo.list_custom_field_definitions_by_project(project_id)?;

    let cat_tag: HashMap<i32, String> = categories.iter().map(|c| (c.id, c.tag.clone())).collect();
    let app_tag: HashMap<i32, String> = applicability
        .iter()
        .map(|c| (c.id, c.tag.clone()))
        .collect();
    let req_st_tag: HashMap<i32, String> =
        req_statuses.iter().map(|c| (c.id, c.tag.clone())).collect();
    let ver_st_tag: HashMap<i32, String> =
        ver_statuses.iter().map(|c| (c.id, c.tag.clone())).collect();
    let method_tag: HashMap<i32, String> = methods.iter().map(|c| (c.id, c.tag.clone())).collect();
    let field_label: HashMap<i32, String> = custom_defs
        .iter()
        .map(|c| (c.id, c.label.clone()))
        .collect();
    let req_code: HashMap<i32, String> = repo
        .get_requirements_by_project(project_id)?
        .into_iter()
        .map(|r| (r.id, r.reference_code.clone()))
        .collect();

    let members = repo
        .get_members_by_project(project_id)?
        .into_iter()
        .filter_map(|m| {
            username(m.user_id).map(|username| BundleMember {
                username,
                role: role_label(m.role).to_string(),
            })
        })
        .collect();

    let reviewers = repo
        .list_project_reviewer_ids(project_id)?
        .into_iter()
        .filter_map(username)
        .collect();

    let mut requirements = Vec::new();
    for req in repo.get_requirements_by_project(project_id)? {
        let method_ids = repo
            .get_verification_method_ids_for_requirement(req.id)
            .unwrap_or_default();
        let mut parent_links = Vec::new();
        if let Some(vid) = req.current_version_id {
            if let Ok(links) = repo.list_links_by_source_version(vid) {
                for link in links {
                    if let Ok(parent_ver) =
                        repo.get_requirement_version_by_id(link.target_version_id)
                    {
                        if let Some(code) = req_code.get(&parent_ver.requirement_id) {
                            parent_links.push(BundleParentLink {
                                reference_code: code.clone(),
                                link_type: link.link_type,
                            });
                        }
                    }
                }
            }
        }
        let mut custom_fields = Vec::new();
        if let Some(vid) = req.current_version_id {
            if let Ok(values) = repo.get_custom_field_values_for_version(vid) {
                for value in values {
                    custom_fields.push(BundleCustomValue {
                        label: field_label
                            .get(&value.field_id)
                            .cloned()
                            .unwrap_or(value.label),
                        value: value.value,
                    });
                }
            }
        }
        requirements.push(BundleRequirement {
            reference_code: req.reference_code,
            title: req.title,
            description: req.description,
            justification: req.justification,
            status_tag: req_st_tag.get(&req.status_id).cloned(),
            category_tag: cat_tag.get(&req.category_id).cloned(),
            applicability_tag: app_tag.get(&req.applicability_id).cloned(),
            author_username: username(req.author_id),
            reviewer_username: username(req.reviewer_id),
            verification_method_tags: method_ids
                .into_iter()
                .filter_map(|id| method_tag.get(&id).cloned())
                .collect(),
            custom_fields,
            parent_links,
        });
    }

    let mut verifications = Vec::new();
    let ver_by_id: HashMap<i32, String> = repo
        .get_verifications_by_project(project_id)?
        .into_iter()
        .map(|v| (v.id, v.reference_code.clone()))
        .collect();
    for ver in repo.get_verifications_by_project(project_id)? {
        verifications.push(BundleVerification {
            reference_code: ver.reference_code,
            name: ver.name,
            description: ver.description,
            source: ver.source,
            status_tag: ver_st_tag.get(&ver.status_id).cloned(),
            verification_method_tag: ver
                .verification_method_id
                .and_then(|id| method_tag.get(&id).cloned()),
            parent_reference_code: ver.parent_id.and_then(|id| ver_by_id.get(&id).cloned()),
            author_username: username(ver.author_id),
            reviewer_username: username(ver.reviewer_id),
        });
    }

    let matrix = repo
        .get_matrix_by_project(project_id)?
        .into_iter()
        .filter_map(|link| {
            Some(BundleMatrixLink {
                requirement_reference_code: req_code.get(&link.req_id)?.clone(),
                verification_reference_code: ver_by_id.get(&link.verification_id)?.clone(),
            })
        })
        .collect();

    let mut comments = Vec::new();
    for req in repo.get_requirements_by_project(project_id)? {
        let code = req.reference_code.clone();
        for comment in repo.list_comments_by_requirement(req.id, None)? {
            comments.push(BundleComment {
                requirement_reference_code: code.clone(),
                body: comment.body,
                author_username: username(comment.author_id),
            });
        }
    }

    Ok(ProjectBundle {
        format: FORMAT_V1.to_string(),
        project: BundleProject {
            name: project.name,
            description: project.description,
            status: Some(project.status.to_db_string().to_string()),
        },
        catalog: BundleCatalog {
            categories: categories
                .into_iter()
                .map(|c| BundleTagged {
                    title: c.title,
                    description: c.description,
                    tag: c.tag,
                })
                .collect(),
            applicability: applicability
                .into_iter()
                .map(|c| BundleTagged {
                    title: c.title,
                    description: c.description,
                    tag: c.tag,
                })
                .collect(),
            requirement_statuses: req_statuses
                .into_iter()
                .map(|c| BundleStatus {
                    title: c.title,
                    description: c.description,
                    tag: c.tag,
                    tag_color: c.tag_color,
                })
                .collect(),
            verification_statuses: ver_statuses
                .into_iter()
                .map(|c| BundleStatus {
                    title: c.title,
                    description: c.description,
                    tag: c.tag,
                    tag_color: c.tag_color,
                })
                .collect(),
            verification_methods: methods
                .into_iter()
                .map(|c| BundleTagged {
                    title: c.title,
                    description: c.description,
                    tag: c.tag,
                })
                .collect(),
            custom_fields: custom_defs
                .into_iter()
                .map(|c| BundleCustomField {
                    label: c.label,
                    field_type: c.field_type,
                    enum_values: c
                        .enum_values
                        .and_then(|v| serde_json::from_value::<Vec<String>>(v).ok()),
                    sort_order: Some(c.sort_order),
                })
                .collect(),
        },
        members,
        reviewers,
        requirements,
        verifications,
        matrix,
        comments,
    })
}

pub fn import_bundle(
    state: &AppState<DieselCachedRepo>,
    actor: &User,
    bundle: ProjectBundle,
    group_id: Option<i32>,
) -> Result<BundleImportResult, RepoError> {
    let mut warnings = Vec::new();
    let mut errors = Vec::new();
    let mut counts = BundleImportedCounts::default();

    let project_service = ProjectService::new(state);
    let status = bundle
        .project
        .status
        .as_deref()
        .and_then(ProjectStatus::from_db_string)
        .unwrap_or(ProjectStatus::Active);
    let project_id = project_service.create(
        actor,
        NewProject {
            name: bundle.project.name.clone(),
            description: bundle.project.description.clone(),
            owner_id: Some(actor.id),
            status: ProjectStatus::Active,
            group_id,
        },
    )?;
    if status != ProjectStatus::Active {
        let project = project_service.get_by_id(project_id)?;
        let _ = project_service.update(
            actor,
            project_id,
            UpdateProject {
                name: project.name,
                description: project.description,
                owner_id: project.owner_id,
                status: Some(status),
                slug: None,
                group_id: project.group_id,
            },
        );
    }

    let catalog = import_catalog(
        state,
        actor,
        project_id,
        &bundle.catalog,
        &mut warnings,
        &mut counts,
    )?;
    import_members(
        state,
        project_id,
        actor,
        &bundle.members,
        &bundle.reviewers,
        &mut warnings,
        &mut counts,
    )?;

    let req_ids = import_requirements(
        state,
        actor,
        project_id,
        &bundle.requirements,
        &catalog,
        &mut warnings,
        &mut errors,
        &mut counts,
    )?;
    let ver_ids = import_verifications(
        state,
        actor,
        project_id,
        &bundle.verifications,
        &catalog,
        &mut warnings,
        &mut errors,
        &mut counts,
    )?;
    import_matrix(
        state,
        actor,
        project_id,
        &bundle.matrix,
        &req_ids,
        &ver_ids,
        &mut warnings,
        &mut errors,
        &mut counts,
    )?;
    import_comments(
        state,
        actor,
        &bundle.comments,
        &req_ids,
        &mut warnings,
        &mut errors,
        &mut counts,
    )?;

    let project = project_service.get_by_id(project_id)?;
    Ok(BundleImportResult {
        project_id,
        slug: project.slug.clone(),
        project_base_path: project_base_path(&project),
        imported_counts: counts,
        warnings,
        errors,
    })
}

struct CatalogMaps {
    categories: HashMap<String, i32>,
    applicability: HashMap<String, i32>,
    req_statuses: HashMap<String, i32>,
    ver_statuses: HashMap<String, i32>,
    methods: HashMap<String, i32>,
    custom_fields: HashMap<String, i32>,
    default_req_status: i32,
    default_ver_status: i32,
    default_category: i32,
    default_applicability: i32,
}

fn import_catalog(
    state: &AppState<DieselCachedRepo>,
    actor: &User,
    project_id: i32,
    catalog: &BundleCatalog,
    warnings: &mut Vec<String>,
    counts: &mut BundleImportedCounts,
) -> Result<CatalogMaps, RepoError> {
    let category_service = CategoryService::new(state);
    let applicability_service = ApplicabilityService::new(state);
    let status_service = StatusService::new(state);
    let custom_service = CustomFieldService::new(state);

    for item in &catalog.categories {
        let existing = state.repo_read().get_categories_by_project(project_id)?;
        if lookup_tagged(&existing, |c| &c.tag, |c| &c.title, &item.tag, &item.title).is_some() {
            continue;
        }
        category_service.create(
            actor,
            NewCategory {
                id: None,
                title: item.title.clone(),
                description: item.description.clone(),
                tag: item.tag.clone(),
                project_id,
            },
        )?;
        counts.categories += 1;
    }
    for item in &catalog.applicability {
        let existing = state.repo_read().get_applicability_by_project(project_id)?;
        if lookup_tagged(&existing, |c| &c.tag, |c| &c.title, &item.tag, &item.title).is_some() {
            continue;
        }
        applicability_service.create(
            actor,
            NewApplicability {
                id: None,
                title: item.title.clone(),
                description: item.description.clone(),
                tag: item.tag.clone(),
                project_id,
            },
        )?;
        counts.applicability += 1;
    }
    for item in &catalog.requirement_statuses {
        let existing = state
            .repo_read()
            .get_requirement_status_by_project(project_id)?;
        if lookup_tagged(&existing, |c| &c.tag, |c| &c.title, &item.tag, &item.title).is_some() {
            continue;
        }
        status_service.create_requirement_status(NewRequirementStatus {
            id: None,
            title: item.title.clone(),
            description: item.description.clone(),
            tag: item.tag.clone(),
            project_id,
            is_system: false,
            tag_color: item.tag_color.clone(),
        })?;
        counts.requirement_statuses += 1;
    }
    for item in &catalog.verification_statuses {
        let existing = state
            .repo_read()
            .get_verification_status_by_project(project_id)?;
        if lookup_tagged(&existing, |c| &c.tag, |c| &c.title, &item.tag, &item.title).is_some() {
            continue;
        }
        status_service.create_verification_status(NewVerificationStatus {
            id: None,
            title: item.title.clone(),
            description: item.description.clone(),
            tag: item.tag.clone(),
            project_id,
            is_system: false,
            tag_color: item.tag_color.clone(),
        })?;
        counts.verification_statuses += 1;
    }
    for item in &catalog.verification_methods {
        let existing = state
            .repo_read()
            .get_verification_methods_by_project(project_id)?;
        if lookup_tagged(&existing, |c| &c.tag, |c| &c.title, &item.tag, &item.title).is_some() {
            continue;
        }
        let mut repo = state.repo_write();
        repo.insert_new_verification_method(&NewVerificationMethod {
            id: None,
            title: item.title.clone(),
            description: item.description.clone(),
            tag: item.tag.clone(),
            project_id,
        })?;
        counts.verification_methods += 1;
    }
    for item in &catalog.custom_fields {
        let existing = state
            .repo_read()
            .list_custom_field_definitions_by_project(project_id)?;
        if existing
            .iter()
            .any(|c| c.label.eq_ignore_ascii_case(&item.label))
        {
            continue;
        }
        custom_service.create(
            project_id,
            CustomFieldDefinitionPayload {
                label: item.label.clone(),
                field_type: item.field_type.clone(),
                enum_values: item.enum_values.clone(),
                sort_order: item.sort_order,
            },
        )?;
        counts.custom_fields += 1;
    }

    let repo = state.repo_read();
    let categories = repo.get_categories_by_project(project_id)?;
    let applicability = repo.get_applicability_by_project(project_id)?;
    let req_statuses = repo.get_requirement_status_by_project(project_id)?;
    let ver_statuses = repo.get_verification_status_by_project(project_id)?;
    let methods = repo.get_verification_methods_by_project(project_id)?;
    let custom_fields = repo.list_custom_field_definitions_by_project(project_id)?;

    let default_category = pick_default(&categories, |c| &c.tag, |c| &c.title, "DEF", "Default")
        .or_else(|| categories.first().map(|c| c.id))
        .ok_or_else(|| RepoError::BadInput("project has no categories".into()))?;
    let default_applicability =
        pick_default(&applicability, |c| &c.tag, |c| &c.title, "DEF", "Default")
            .or_else(|| applicability.first().map(|c| c.id))
            .ok_or_else(|| RepoError::BadInput("project has no applicability".into()))?;
    let default_req_status = pick_default(&req_statuses, |c| &c.tag, |c| &c.title, "Drf", "Draft")
        .or_else(|| req_statuses.first().map(|c| c.id))
        .ok_or_else(|| RepoError::BadInput("project has no requirement statuses".into()))?;
    let default_ver_status =
        pick_default(&ver_statuses, |c| &c.tag, |c| &c.title, "Pend", "Pending")
            .or_else(|| ver_statuses.first().map(|c| c.id))
            .ok_or_else(|| RepoError::BadInput("project has no verification statuses".into()))?;

    if categories.is_empty() {
        warnings.push("no categories after catalog import".into());
    }

    Ok(CatalogMaps {
        categories: index_tagged(&categories, |c| c.id, |c| &c.tag, |c| &c.title),
        applicability: index_tagged(&applicability, |c| c.id, |c| &c.tag, |c| &c.title),
        req_statuses: index_tagged(&req_statuses, |c| c.id, |c| &c.tag, |c| &c.title),
        ver_statuses: index_tagged(&ver_statuses, |c| c.id, |c| &c.tag, |c| &c.title),
        methods: index_tagged(&methods, |c| c.id, |c| &c.tag, |c| &c.title),
        custom_fields: custom_fields
            .into_iter()
            .map(|c| (c.label.to_lowercase(), c.id))
            .collect(),
        default_req_status,
        default_ver_status,
        default_category,
        default_applicability,
    })
}

fn import_members(
    state: &AppState<DieselCachedRepo>,
    project_id: i32,
    actor: &User,
    members: &[BundleMember],
    reviewers: &[String],
    warnings: &mut Vec<String>,
    counts: &mut BundleImportedCounts,
) -> Result<(), RepoError> {
    let users = state.repo_read().get_users_all()?;
    for member in members {
        let Some(user) = find_user(&users, &member.username) else {
            warnings.push(format!(
                "skipped member '{}': user does not exist on this instance",
                member.username
            ));
            continue;
        };
        if user.id == actor.id {
            continue;
        }
        let already = state
            .repo_read()
            .get_members_by_project(project_id)?
            .iter()
            .any(|m| m.user_id == user.id);
        if already {
            continue;
        }
        state.repo_write().add_project_member(&NewProjectMember {
            project_id,
            user_id: user.id,
            role: parse_role(&member.role),
        })?;
        counts.members += 1;
    }
    let member_ids: HashSet<i32> = state
        .repo_read()
        .get_members_by_project(project_id)?
        .into_iter()
        .map(|m| m.user_id)
        .collect();
    let reviewer_ids: Vec<i32> = reviewers
        .iter()
        .filter_map(|name| {
            let user = find_user(&users, name)?;
            if member_ids.contains(&user.id) {
                Some(user.id)
            } else {
                warnings.push(format!(
                    "skipped reviewer '{name}': not a member of the imported project"
                ));
                None
            }
        })
        .collect();
    if !reviewer_ids.is_empty() {
        counts.reviewers = reviewer_ids.len();
        state
            .repo_write()
            .replace_project_reviewers(project_id, &reviewer_ids)?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn import_requirements(
    state: &AppState<DieselCachedRepo>,
    actor: &User,
    project_id: i32,
    items: &[BundleRequirement],
    catalog: &CatalogMaps,
    warnings: &mut Vec<String>,
    errors: &mut Vec<String>,
    counts: &mut BundleImportedCounts,
) -> Result<HashMap<String, i32>, RepoError> {
    let users = state.repo_read().get_users_all()?;
    let ordered = topo_by_parents(
        items,
        |r| r.reference_code.clone(),
        |r| {
            r.parent_links
                .iter()
                .map(|p| p.reference_code.clone())
                .collect()
        },
    );
    let service = RequirementService::new(state);
    let mut ids = HashMap::new();
    for req in ordered {
        let status_id = resolve_tag(
            req.status_tag.as_deref(),
            &catalog.req_statuses,
            catalog.default_req_status,
        );
        let category_id = resolve_tag(
            req.category_tag.as_deref(),
            &catalog.categories,
            catalog.default_category,
        );
        let applicability_id = resolve_tag(
            req.applicability_tag.as_deref(),
            &catalog.applicability,
            catalog.default_applicability,
        );
        let author_id = resolve_user(&users, req.author_username.as_deref(), actor.id);
        let reviewer_id = resolve_user(&users, req.reviewer_username.as_deref(), actor.id);
        let method_ids: Vec<i32> = req
            .verification_method_tags
            .iter()
            .filter_map(|tag| catalog.methods.get(&tag.to_lowercase()).copied())
            .collect();
        let custom_fields: Vec<CustomFieldValueInput> = req
            .custom_fields
            .iter()
            .filter_map(|value| {
                catalog
                    .custom_fields
                    .get(&value.label.to_lowercase())
                    .map(|field_id| CustomFieldValueInput {
                        field_id: *field_id,
                        value: value.value.clone(),
                    })
            })
            .collect();
        let parent_links: Vec<(i32, String, Option<String>)> = req
            .parent_links
            .iter()
            .filter_map(|link| {
                let parent_id = ids.get(&link.reference_code)?;
                let parent = state.repo_read().get_requirement_by_id(*parent_id).ok()?;
                let version_id = parent.current_version_id?;
                let link_type = if link.link_type.trim().is_empty() {
                    "DERIVES_FROM".to_string()
                } else {
                    link.link_type.clone()
                };
                Some((version_id, link_type, None))
            })
            .collect();
        let payload = NewRequirement {
            id: None,
            title: req.title.clone(),
            description: req.description.clone(),
            author_id,
            category_id,
            status_id,
            reference_code: req.reference_code.clone(),
            reviewer_id,
            applicability_id,
            justification: req.justification.clone(),
            project_id,
        };
        match service.create(
            actor,
            payload.clone(),
            &method_ids,
            if custom_fields.is_empty() {
                None
            } else {
                Some(&custom_fields)
            },
            if parent_links.is_empty() {
                None
            } else {
                Some(parent_links.clone())
            },
        ) {
            Ok(id) => {
                ids.insert(req.reference_code.clone(), id);
                counts.requirements += 1;
            }
            Err(err) => {
                let draft = NewRequirement {
                    status_id: catalog.default_req_status,
                    ..payload
                };
                match service.create(
                    actor,
                    draft,
                    &method_ids,
                    if custom_fields.is_empty() {
                        None
                    } else {
                        Some(&custom_fields)
                    },
                    if parent_links.is_empty() {
                        None
                    } else {
                        Some(parent_links)
                    },
                ) {
                    Ok(id) => {
                        warnings.push(format!(
                            "requirement '{}' imported as Draft: {err}",
                            req.reference_code
                        ));
                        ids.insert(req.reference_code.clone(), id);
                        counts.requirements += 1;
                    }
                    Err(err2) => {
                        errors.push(format!("requirement '{}': {err2}", req.reference_code))
                    }
                }
            }
        }
    }
    Ok(ids)
}

#[allow(clippy::too_many_arguments)]
fn import_verifications(
    state: &AppState<DieselCachedRepo>,
    actor: &User,
    project_id: i32,
    items: &[BundleVerification],
    catalog: &CatalogMaps,
    warnings: &mut Vec<String>,
    errors: &mut Vec<String>,
    counts: &mut BundleImportedCounts,
) -> Result<HashMap<String, i32>, RepoError> {
    let users = state.repo_read().get_users_all()?;
    let ordered = topo_by_parents(
        items,
        |v| v.reference_code.clone(),
        |v| {
            v.parent_reference_code
                .clone()
                .into_iter()
                .collect::<Vec<_>>()
        },
    );
    let service = VerificationService::new(state);
    let mut ids = HashMap::new();
    for ver in ordered {
        let status_id = resolve_tag(
            ver.status_tag.as_deref(),
            &catalog.ver_statuses,
            catalog.default_ver_status,
        );
        let payload = NewVerification {
            id: None,
            reference_code: ver.reference_code.clone(),
            name: ver.name.clone(),
            description: ver.description.clone(),
            source: ver.source.clone(),
            status_id,
            parent_id: ver
                .parent_reference_code
                .as_ref()
                .and_then(|code| ids.get(code).copied()),
            project_id,
            verification_method_id: ver
                .verification_method_tag
                .as_ref()
                .and_then(|tag| catalog.methods.get(&tag.to_lowercase()).copied()),
            author_id: resolve_user(&users, ver.author_username.as_deref(), actor.id),
            reviewer_id: resolve_user(&users, ver.reviewer_username.as_deref(), actor.id),
        };
        match service.create(actor, payload.clone()) {
            Ok(id) => {
                ids.insert(ver.reference_code.clone(), id);
                counts.verifications += 1;
            }
            Err(err) => {
                let pending = NewVerification {
                    status_id: catalog.default_ver_status,
                    ..payload
                };
                match service.create(actor, pending) {
                    Ok(id) => {
                        warnings.push(format!(
                            "verification '{}' imported as Pending: {err}",
                            ver.reference_code
                        ));
                        ids.insert(ver.reference_code.clone(), id);
                        counts.verifications += 1;
                    }
                    Err(err2) => {
                        errors.push(format!("verification '{}': {err2}", ver.reference_code))
                    }
                }
            }
        }
    }
    Ok(ids)
}

#[allow(clippy::too_many_arguments)]
fn import_matrix(
    state: &AppState<DieselCachedRepo>,
    actor: &User,
    project_id: i32,
    links: &[BundleMatrixLink],
    req_ids: &HashMap<String, i32>,
    ver_ids: &HashMap<String, i32>,
    warnings: &mut Vec<String>,
    errors: &mut Vec<String>,
    counts: &mut BundleImportedCounts,
) -> Result<(), RepoError> {
    let service = MatrixService::new(state);
    for link in links {
        let Some(req_id) = req_ids.get(&link.requirement_reference_code) else {
            warnings.push(format!(
                "skipped matrix link: requirement '{}' not imported",
                link.requirement_reference_code
            ));
            continue;
        };
        let Some(ver_id) = ver_ids.get(&link.verification_reference_code) else {
            warnings.push(format!(
                "skipped matrix link: verification '{}' not imported",
                link.verification_reference_code
            ));
            continue;
        };
        match service.link(actor, *req_id, *ver_id, project_id) {
            Ok(()) => counts.matrix_links += 1,
            Err(err) => errors.push(format!(
                "matrix {}→{}: {err}",
                link.requirement_reference_code, link.verification_reference_code
            )),
        }
    }
    Ok(())
}

fn import_comments(
    state: &AppState<DieselCachedRepo>,
    actor: &User,
    comments: &[BundleComment],
    req_ids: &HashMap<String, i32>,
    warnings: &mut Vec<String>,
    errors: &mut Vec<String>,
    counts: &mut BundleImportedCounts,
) -> Result<(), RepoError> {
    let users = state.repo_read().get_users_all()?;
    for comment in comments {
        let Some(req_id) = req_ids.get(&comment.requirement_reference_code) else {
            warnings.push(format!(
                "skipped comment on '{}': requirement not imported",
                comment.requirement_reference_code
            ));
            continue;
        };
        let req = state.repo_read().get_requirement_by_id(*req_id)?;
        let author_id = resolve_user(&users, comment.author_username.as_deref(), actor.id);
        match state
            .repo_write()
            .insert_requirement_comment(&NewRequirementComment {
                requirement_id: *req_id,
                requirement_version_id: req.current_version_id,
                author_id,
                body: comment.body.clone(),
                mcp_idempotency_identity: None,
            }) {
            Ok(_) => counts.comments += 1,
            Err(err) => errors.push(format!(
                "comment on '{}': {err}",
                comment.requirement_reference_code
            )),
        }
    }
    Ok(())
}

fn role_label(role: i32) -> &'static str {
    match role {
        ROLE_ADMIN => "admin",
        ROLE_REVIEWER => "reviewer",
        ROLE_AUTHOR => "author",
        _ => "viewer",
    }
}

fn parse_role(role: &str) -> i32 {
    match role.trim().to_ascii_lowercase().as_str() {
        "admin" | "1" => ROLE_ADMIN,
        "reviewer" | "2" => ROLE_REVIEWER,
        "author" | "3" => ROLE_AUTHOR,
        _ => ROLE_VIEWER,
    }
}

fn names_match(a: &str, b: &str) -> bool {
    a.trim().eq_ignore_ascii_case(b.trim())
}

fn find_user<'a>(users: &'a [User], name: &str) -> Option<&'a User> {
    users.iter().find(|u| {
        names_match(&u.username, name) || names_match(&u.email, name) || names_match(&u.name, name)
    })
}

fn resolve_user(users: &[User], name: Option<&str>, fallback: i32) -> i32 {
    name.and_then(|n| find_user(users, n).map(|u| u.id))
        .unwrap_or(fallback)
}

fn lookup_tagged<T>(
    items: &[T],
    tag: impl Fn(&T) -> &str,
    title: impl Fn(&T) -> &str,
    want_tag: &str,
    want_title: &str,
) -> Option<i32>
where
    T: TaggedId,
{
    items
        .iter()
        .find(|item| names_match(tag(item), want_tag) || names_match(title(item), want_title))
        .map(|item| item.id())
}

trait TaggedId {
    fn id(&self) -> i32;
}

impl TaggedId for crate::models::Category {
    fn id(&self) -> i32 {
        self.id
    }
}
impl TaggedId for crate::models::Applicability {
    fn id(&self) -> i32 {
        self.id
    }
}
impl TaggedId for crate::models::RequirementStatus {
    fn id(&self) -> i32 {
        self.id
    }
}
impl TaggedId for crate::models::VerificationStatus {
    fn id(&self) -> i32 {
        self.id
    }
}
impl TaggedId for crate::models::VerificationMethod {
    fn id(&self) -> i32 {
        self.id
    }
}

fn pick_default<T: TaggedId>(
    items: &[T],
    tag: impl Fn(&T) -> &str,
    title: impl Fn(&T) -> &str,
    want_tag: &str,
    want_title: &str,
) -> Option<i32> {
    lookup_tagged(items, tag, title, want_tag, want_title)
}

fn index_tagged<T: TaggedId>(
    items: &[T],
    id: impl Fn(&T) -> i32,
    tag: impl Fn(&T) -> &str,
    title: impl Fn(&T) -> &str,
) -> HashMap<String, i32> {
    let mut map = HashMap::new();
    for item in items {
        map.insert(tag(item).to_lowercase(), id(item));
        map.insert(title(item).to_lowercase(), id(item));
    }
    map
}

fn resolve_tag(tag: Option<&str>, map: &HashMap<String, i32>, fallback: i32) -> i32 {
    tag.and_then(|t| map.get(&t.to_lowercase()).copied())
        .unwrap_or(fallback)
}

fn topo_by_parents<T>(
    items: &[T],
    code: impl Fn(&T) -> String,
    parents: impl Fn(&T) -> Vec<String>,
) -> Vec<&T> {
    let codes: HashSet<String> = items.iter().map(&code).collect();
    let mut remaining: Vec<&T> = items.iter().collect();
    let mut done: HashSet<String> = HashSet::new();
    let mut ordered = Vec::new();
    while !remaining.is_empty() {
        let mut ready = Vec::new();
        let mut rest = Vec::new();
        for item in remaining {
            let ok = parents(item)
                .into_iter()
                .all(|p| p.trim().is_empty() || done.contains(&p) || !codes.contains(&p));
            if ok {
                ready.push(item);
            } else {
                rest.push(item);
            }
        }
        if ready.is_empty() {
            ordered.extend(rest);
            break;
        }
        for item in ready {
            done.insert(code(item));
            ordered.push(item);
        }
        remaining = rest;
    }
    ordered
}
