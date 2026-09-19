// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Service for ReqIF 1.2 export and import.

use crate::app::{AppState, DieselCachedRepo};
use crate::models::{NewRequirement, User};
use crate::repository::errors::RepoError;
use crate::repository::{RequirementCommentsRepository, UserRepository};
use crate::reqif::import::{object_to_fields, parse_reqif, ImportConfig, ImportResult};
use crate::reqif::mapping;
use crate::reqif::to_reqif;
use crate::services::{BaselineService, ProjectService, RequirementService, StatusService};
use std::collections::HashMap;

fn is_valid_marreq_reference(reference: &str) -> bool {
    let mut parts = reference.split('-');
    let Some(prefix) = parts.next() else {
        return false;
    };
    if !(2..=4).contains(&prefix.len()) || !prefix.chars().all(|c| c.is_ascii_uppercase()) {
        return false;
    }
    let mut has_suffix = false;
    for part in parts {
        has_suffix = true;
        if part.is_empty()
            || part.len() > 6
            || !part
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        {
            return false;
        }
    }
    has_suffix
}

fn reqif_relation_link_type(type_ref: &str) -> &'static str {
    let normalized = type_ref.to_ascii_lowercase();
    if normalized.contains("derive") {
        "DERIVES_FROM"
    } else if normalized.contains("refine") {
        "REFINES"
    } else if normalized.contains("depend") {
        "DEPENDS_ON"
    } else if normalized.contains("satisf")
        || normalized.contains("fulfill")
        || normalized.contains("implement")
    {
        "SATISFIES"
    } else {
        "RELATES_TO"
    }
}

fn create_import_link(
    req_service: &RequirementService<'_>,
    imported: &HashMap<String, i32>,
    source_reqif_id: &str,
    target_reqif_id: &str,
    link_type: &str,
    project_id: i32,
) -> Result<(), String> {
    let source_id = imported
        .get(source_reqif_id)
        .copied()
        .ok_or_else(|| format!("source SpecObject '{source_reqif_id}' was not imported"))?;
    let target_id = imported
        .get(target_reqif_id)
        .copied()
        .ok_or_else(|| format!("target SpecObject '{target_reqif_id}' was not imported"))?;
    let source = req_service
        .get_by_id(source_id)
        .map_err(|e| format!("could not load imported source {source_reqif_id}: {e}"))?;
    let target = req_service
        .get_by_id(target_id)
        .map_err(|e| format!("could not load imported target {target_reqif_id}: {e}"))?;
    let source_version = source
        .current_version_id
        .ok_or_else(|| format!("imported source {source_reqif_id} has no current version"))?;
    let target_version = target
        .current_version_id
        .ok_or_else(|| format!("imported target {target_reqif_id} has no current version"))?;
    req_service
        .create_requirement_version_link(
            source_version,
            target_version,
            link_type,
            project_id,
            None,
            None,
        )
        .map(|_| ())
        .map_err(|e| format!("{source_reqif_id} -> {target_reqif_id} ({link_type}): {e}"))
}

pub struct ReqIFService<'a> {
    state: &'a AppState<DieselCachedRepo>,
}

impl<'a> ReqIFService<'a> {
    pub fn new(state: &'a AppState<DieselCachedRepo>) -> Self {
        Self { state }
    }

    /// Export project requirements as ReqIF 1.2 XML (includes comments as Remarks when present).
    pub fn export_project(&self, project_id: i32) -> Result<String, RepoError> {
        let req_service = RequirementService::new(self.state);
        let project_service = ProjectService::new(self.state);
        let project = project_service.get_by_id(project_id)?;
        let requirements = req_service.list_by_project(project_id)?;
        let parent_map: HashMap<i32, i32> = requirements
            .iter()
            .filter_map(|r| {
                r.current_version_id.and_then(|vid| {
                    req_service
                        .get_parent_requirement_ids_for_version(vid)
                        .into_iter()
                        .next()
                        .map(|pid| (r.id, pid))
                })
            })
            .collect();
        let comments_map = self.build_comments_map(&requirements);
        Ok(to_reqif(
            &project.name,
            &requirements,
            &parent_map,
            Some(&comments_map),
        ))
    }

    /// Export a baseline's requirements as ReqIF 1.2 XML (immutable snapshot).
    pub fn export_baseline(&self, project_id: i32, baseline_id: i32) -> Result<String, RepoError> {
        let project_service = ProjectService::new(self.state);
        let baseline_service = BaselineService::new(self.state);
        let project = project_service.get_by_id(project_id)?;
        let baseline = baseline_service.get_by_id(baseline_id)?;
        if baseline.project_id != project_id {
            return Err(RepoError::NotFound);
        }
        let requirements = baseline_service.get_requirements(baseline_id)?;
        let req_service = RequirementService::new(self.state);
        let parent_map: HashMap<i32, i32> = requirements
            .iter()
            .filter_map(|r| {
                r.current_version_id.and_then(|vid| {
                    req_service
                        .get_parent_requirement_ids_for_version(vid)
                        .into_iter()
                        .next()
                        .map(|pid| (r.id, pid))
                })
            })
            .collect();
        let title = format!("{} (baseline: {})", project.name, baseline.name);
        let comments_map = self.build_comments_map(&requirements);
        Ok(to_reqif(
            &title,
            &requirements,
            &parent_map,
            Some(&comments_map),
        ))
    }

    /// Build requirement_id -> "Author, date: body\n..." for ReqIF Remarks.
    fn build_comments_map(
        &self,
        requirements: &[crate::models::Requirement],
    ) -> HashMap<i32, String> {
        let repo = self.state.repo_read();
        let mut map = HashMap::new();
        for req in requirements {
            let comments = match repo.list_comments_by_requirement(req.id, None) {
                Ok(c) => c,
                Err(_) => continue,
            };
            if comments.is_empty() {
                continue;
            }
            let lines: Vec<String> = comments
                .iter()
                .map(|c| {
                    let author = repo
                        .get_user_by_id(c.author_id)
                        .ok()
                        .map(|u| u.name)
                        .unwrap_or_else(|| format!("User#{}", c.author_id));
                    format!(
                        "{}, {}: {}",
                        author,
                        c.created_at.format("%Y-%m-%d %H:%M"),
                        c.body
                    )
                })
                .collect();
            map.insert(req.id, lines.join("\n"));
        }
        map
    }

    /// Import ReqIF XML into a project. Creates requirements in topological order (parents before children).
    pub fn import_into_project(
        &self,
        xml: &[u8],
        config: &ImportConfig,
        actor: &User,
    ) -> Result<ImportResult, String> {
        let doc = parse_reqif(xml)?;
        let mut preflight_errors = Vec::new();
        let mut preflight_warnings = doc.warnings.clone();
        let mut object_ids = std::collections::HashSet::new();
        let mut valid_references = std::collections::HashSet::new();
        for obj in &doc.objects {
            if obj.id.is_empty() {
                preflight_errors.push("SPEC-OBJECT without IDENTIFIER".into());
            } else if !object_ids.insert(obj.id.clone()) {
                preflight_errors.push(format!("duplicate SPEC-OBJECT IDENTIFIER '{}'", obj.id));
            }
            if let Some(reference) = object_to_fields(obj).1 {
                if is_valid_marreq_reference(&reference)
                    && !valid_references.insert(reference.clone())
                {
                    preflight_errors.push(format!(
                        "duplicate requirement reference '{}' in ReqIF document",
                        reference
                    ));
                }
            }
        }
        let mut planned_references = HashMap::new();
        let mut fallback_number = 1usize;
        for obj in &doc.objects {
            let mapped = object_to_fields(obj).1;
            let reference = if let Some(reference) =
                mapped.filter(|reference| is_valid_marreq_reference(reference))
            {
                reference
            } else {
                loop {
                    let candidate = format!("REQ-{fallback_number:04}");
                    fallback_number += 1;
                    if valid_references.insert(candidate.clone()) {
                        break candidate;
                    }
                }
            };
            planned_references.insert(obj.id.clone(), reference);
        }
        for edge in &doc.hierarchy_edges {
            if !object_ids.contains(&edge.child_id) {
                preflight_errors.push(format!(
                    "SPEC-HIERARCHY references missing child SpecObject '{}'",
                    edge.child_id
                ));
            }
            if !object_ids.contains(&edge.parent_id) {
                preflight_errors.push(format!(
                    "SPEC-HIERARCHY references missing parent SpecObject '{}'",
                    edge.parent_id
                ));
            }
        }
        for relation in &doc.relations {
            if !object_ids.contains(&relation.source) {
                preflight_warnings.push(format!(
                    "SPEC-RELATION '{}' references missing source SpecObject '{}'",
                    relation.id, relation.source
                ));
            }
            if !object_ids.contains(&relation.target) {
                preflight_warnings.push(format!(
                    "SPEC-RELATION '{}' references missing target SpecObject '{}'",
                    relation.id, relation.target
                ));
            }
        }
        if !preflight_errors.is_empty() {
            return Ok(ImportResult {
                success: false,
                message: format!(
                    "ReqIF preflight failed with {} error(s); nothing was imported",
                    preflight_errors.len()
                ),
                imported_count: 0,
                created_link_count: 0,
                errors: preflight_errors,
                warnings: preflight_warnings,
                imported_requirement_ids: Vec::new(),
            });
        }

        let req_service = RequirementService::new(self.state);
        let status_service = StatusService::new(self.state);
        let statuses = status_service
            .list_requirement_statuses_by_project(config.project_id)
            .map_err(|e| e.to_string())?;
        let default_status_id = if statuses.is_empty() {
            config.default_status_id
        } else {
            statuses
                .first()
                .map(|s| s.id)
                .unwrap_or(config.default_status_id)
        };

        let mut hierarchy_pairs = std::collections::BTreeSet::new();
        for edge in &doc.hierarchy_edges {
            if edge.child_id != edge.parent_id {
                hierarchy_pairs.insert((edge.child_id.clone(), edge.parent_id.clone()));
            }
        }

        let mut reqif_id_to_marreq_id: HashMap<String, i32> = HashMap::new();
        let mut imported_count = 0usize;
        let mut created_link_count = 0usize;
        let mut errors = Vec::new();
        let mut warnings = preflight_warnings;
        let mut imported_requirement_ids = Vec::new();
        if doc.object_type_count > 0 {
            warnings.push(format!(
                "{} SPEC-OBJECT-TYPE definition(s) were collapsed into Marreq requirements",
                doc.object_type_count
            ));
        }
        if doc.datatype_definition_count > 0 {
            warnings.push(format!(
                "{} datatype definition(s) were used for parsing but were not persisted",
                doc.datatype_definition_count
            ));
        }
        if doc.specification_count > 1 {
            warnings.push(format!(
                "{} specifications were merged into one Marreq project",
                doc.specification_count
            ));
        }
        if doc.xhtml_value_count > 0 {
            warnings.push(format!(
                "{} XHTML value(s) were converted to plain text; formatting may be lost",
                doc.xhtml_value_count
            ));
        }
        if doc.enumeration_value_count > 0 {
            warnings.push(format!(
                "{} enumeration value(s) were parsed as text but not persisted as enum custom fields",
                doc.enumeration_value_count
            ));
        }
        if doc.scalar_value_count > 0 {
            warnings.push(format!(
                "{} integer/real/boolean/date value(s) were parsed as text but not persisted as typed custom fields",
                doc.scalar_value_count
            ));
        }
        if doc.objects.iter().any(|obj| obj.last_change.is_some()) {
            warnings.push("ReqIF LAST-CHANGE timestamps were not persisted".into());
        }
        let extra: Vec<String> = doc
            .objects
            .iter()
            .flat_map(|obj| obj.attributes.keys())
            .filter(|name| !mapping::is_core_field(name))
            .cloned()
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        if !extra.is_empty() {
            warnings.push(format!(
                "{} ReqIF attribute(s) were parsed but not stored as Marreq fields: {}",
                extra.len(),
                extra.join(", ")
            ));
        }

        // Pass 1: create all requirements (no parent links yet)
        for obj in &doc.objects {
            let (title_opt, _ref_opt, desc_opt, status_opt, justification_opt) =
                object_to_fields(obj);

            let missing_title = title_opt.is_none();
            let mut title = title_opt.unwrap_or_else(|| "Imported Requirement".to_string());
            if missing_title {
                warnings.push(format!(
                    "{}: no Title/ReqIF.Name/LONG-NAME; using fallback title",
                    obj.id
                ));
            }
            if title.trim().len() < 3 {
                title = format!("Imported {}", obj.id);
                if title.len() < 3 {
                    title = "Imported Requirement".into();
                }
            }
            if title.len() > 255 {
                title.truncate(255);
            }
            let reference_code = planned_references
                .get(&obj.id)
                .cloned()
                .unwrap_or_else(|| format!("REQ-{:04}", imported_count + 1));
            let mut description = desc_opt.unwrap_or_default();
            if description.trim().is_empty() {
                description = title.clone();
            }
            if description.len() > 2000 {
                description.truncate(2000);
                warnings.push(format!(
                    "{}: description truncated to 2000 characters",
                    obj.id
                ));
            }
            let status_id = status_opt
                .and_then(|s| {
                    statuses
                        .iter()
                        .find(|st| st.title.eq_ignore_ascii_case(s.trim()))
                        .map(|st| st.id)
                })
                .unwrap_or(default_status_id);
            let justification = justification_opt.filter(|s| !s.is_empty());

            let payload = NewRequirement {
                id: None,
                title: title.clone(),
                description,
                author_id: config.author_id,
                category_id: config.default_category_id,
                status_id,
                reference_code: reference_code.clone(),
                reviewer_id: config.reviewer_id,
                applicability_id: config.default_applicability_id,
                justification,
                project_id: config.project_id,
            };

            let verification_method_ids = [config.default_verification_method_id];
            match req_service.create(actor, payload, &verification_method_ids, None, None) {
                Ok(id) => {
                    imported_count += 1;
                    imported_requirement_ids.push(id);
                    reqif_id_to_marreq_id.insert(obj.id.clone(), id);
                }
                Err(e) => {
                    errors.push(format!("{} ({}): {}", reference_code, title, e));
                }
            }
        }

        // Pass 2a: hierarchy is structural and must be preserved.
        for (child_reqif_id, parent_reqif_id) in &hierarchy_pairs {
            match create_import_link(
                &req_service,
                &reqif_id_to_marreq_id,
                child_reqif_id,
                parent_reqif_id,
                "DERIVES_FROM",
                config.project_id,
            ) {
                Ok(()) => created_link_count += 1,
                Err(e) => errors.push(format!("hierarchy link failed: {e}")),
            }
        }

        // Pass 2b: ReqIF trace relations are typed independently from hierarchy.
        // Marreq stores both in one acyclic graph, so a relation that would form
        // a cycle cannot be represented; report it rather than silently dropping it.
        for rel in &doc.relations {
            if rel.source.is_empty() || rel.target.is_empty() || rel.source == rel.target {
                warnings.push(format!(
                    "SPEC-RELATION '{}' has an empty or self reference and was skipped",
                    rel.id
                ));
                continue;
            }
            if !object_ids.contains(&rel.source) || !object_ids.contains(&rel.target) {
                continue;
            }
            if hierarchy_pairs.contains(&(rel.source.clone(), rel.target.clone())) {
                warnings.push(format!(
                    "SPEC-RELATION '{}' duplicates a hierarchy edge; its distinct type '{}' was not preserved",
                    rel.id, rel.type_ref
                ));
                continue;
            }
            let link_type = reqif_relation_link_type(&rel.type_ref);
            match create_import_link(
                &req_service,
                &reqif_id_to_marreq_id,
                &rel.source,
                &rel.target,
                link_type,
                config.project_id,
            ) {
                Ok(()) => created_link_count += 1,
                Err(e) => warnings.push(format!(
                    "SPEC-RELATION '{}' could not be represented and was skipped: {e}",
                    rel.id
                )),
            }
        }

        Ok(ImportResult {
            success: errors.is_empty(),
            message: if errors.is_empty() {
                format!(
                    "Successfully imported {} requirements ({} links)",
                    imported_count, created_link_count
                )
            } else {
                format!(
                    "Imported {} requirements with {} errors",
                    imported_count,
                    errors.len()
                )
            },
            imported_count,
            created_link_count,
            errors,
            warnings,
            imported_requirement_ids,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{AppState, DieselCachedRepo};
    use crate::repository::diesel_repo_mock::DieselRepoMock;
    use crate::repository::CacheRepository;
    use crate::status_enums::ProjectStatus;
    use chrono::{NaiveDate, NaiveDateTime};
    use std::sync::{Arc, RwLock};

    fn epoch() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(1970, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
    }

    #[test]
    fn reqif_service_new_constructs() {
        let mock = DieselRepoMock::default();
        let cached = CacheRepository::new(mock, 0);
        let state = AppState {
            repo: Arc::new(RwLock::new(cached)),
        };
        let _service = ReqIFService::new(&state);
    }

    #[test]
    fn export_project_returns_xml_with_project_name_and_requirements() {
        let mut mock = DieselRepoMock::default();
        let proj = crate::models::Project {
            id: 1,
            name: "Export Test Project".to_string(),
            description: Some("Desc".into()),
            creation_date: Some(epoch()),
            update_date: Some(epoch()),
            status: ProjectStatus::Active,
            owner_id: Some(1),
            slug: "export-test-project".into(),
            group_id: None,
        };
        mock.projects.insert(1, proj);
        mock.requirement_statuses.insert(
            1,
            crate::models::RequirementStatus {
                id: 1,
                title: "Draft".into(),
                description: "".into(),
                tag: "D".into(),
                project_id: 1,
                is_system: false,
                tag_color: None,
            },
        );
        let req = crate::models::Requirement {
            id: 10,
            current_version_id: None,
            same_as_current: None,
            title: "Req Title".into(),
            description: "Desc".into(),
            status_id: 1,
            author_id: 1,
            reviewer_id: 1,
            reference_code: "REQ-001".into(),
            category_id: 1,
            parent_id: None,
            creation_date: epoch(),
            update_date: epoch(),
            deadline_date: None,
            applicability_id: 1,
            justification: None,
            project_id: 1,
            approval_state: "draft".to_string(),
            approved_by: None,
            approved_at: None,
            custom_fields: None,
        };
        mock.requirements.insert(10, req);

        let cached = CacheRepository::new(mock, 0);
        let state = AppState::<DieselCachedRepo> {
            repo: Arc::new(RwLock::new(cached)),
        };
        let service = ReqIFService::new(&state);
        let xml = service.export_project(1).unwrap();
        assert!(xml.contains("Export Test Project"));
        assert!(xml.contains("Req Title"));
        assert!(xml.contains("REQ-001"));
        assert!(xml.contains("REQ-IF"));
    }

    #[test]
    fn export_project_not_found_returns_err() {
        let mock = DieselRepoMock::default();
        let cached = CacheRepository::new(mock, 0);
        let state = AppState::<DieselCachedRepo> {
            repo: Arc::new(RwLock::new(cached)),
        };
        let service = ReqIFService::new(&state);
        let result = service.export_project(999);
        assert!(result.is_err());
    }

    #[test]
    fn export_baseline_returns_xml_with_baseline_name() {
        let mut mock = DieselRepoMock::default();
        mock.projects.insert(
            1,
            crate::models::Project {
                id: 1,
                name: "Proj".to_string(),
                description: None,
                creation_date: Some(epoch()),
                update_date: Some(epoch()),
                status: ProjectStatus::Active,
                owner_id: Some(1),
                slug: "proj".into(),
                group_id: None,
            },
        );
        mock.baselines.push(crate::models::Baseline {
            id: 1,
            project_id: 1,
            name: "v1.0".to_string(),
            description: None,
            created_at: epoch(),
            created_by: 1,
            source_saved_view_id: None,
            source_view_definition: None,
        });
        mock.requirement_statuses.insert(
            1,
            crate::models::RequirementStatus {
                id: 1,
                title: "Draft".into(),
                description: "".into(),
                tag: "D".into(),
                project_id: 1,
                is_system: false,
                tag_color: None,
            },
        );
        mock.requirement_versions.insert(
            10,
            crate::models::RequirementVersion {
                id: 10,
                requirement_id: 1,
                title: "Req".into(),
                description: "D".into(),
                status_id: 1,
                author_id: 1,
                reviewer_id: 1,
                category_id: 1,
                applicability_id: 1,
                justification: None,
                deadline_date: None,
                created_at: epoch(),
                approval_state: "draft".into(),
                approved_by: None,
                approved_at: None,
                reviewed_by: None,
                reviewed_at: None,
            },
        );
        mock.requirements.insert(
            1,
            crate::models::Requirement {
                id: 1,
                current_version_id: Some(10),
                same_as_current: None,
                title: "Req".into(),
                description: "D".into(),
                status_id: 1,
                author_id: 1,
                reviewer_id: 1,
                reference_code: "R-1".into(),
                category_id: 1,
                parent_id: None,
                creation_date: epoch(),
                update_date: epoch(),
                deadline_date: None,
                applicability_id: 1,
                justification: None,
                project_id: 1,
                approval_state: "draft".into(),
                approved_by: None,
                approved_at: None,
                custom_fields: None,
            },
        );
        mock.baseline_requirements
            .push(crate::models::BaselineRequirement {
                baseline_id: 1,
                requirement_id: 1,
                version_id: 10,
            });
        let cached = CacheRepository::new(mock, 0);
        let state = AppState::<DieselCachedRepo> {
            repo: Arc::new(RwLock::new(cached)),
        };
        let service = ReqIFService::new(&state);
        let xml = service.export_baseline(1, 1).unwrap();
        assert!(xml.contains("REQ-IF"));
        assert!(xml.contains("v1.0"));
        assert!(xml.contains("baseline"));
        assert!(xml.contains("Req"));
    }

    #[test]
    fn export_baseline_wrong_project_returns_err() {
        let mut mock = DieselRepoMock::default();
        mock.projects.insert(
            1,
            crate::models::Project {
                id: 1,
                name: "P1".into(),
                description: None,
                creation_date: None,
                update_date: None,
                status: ProjectStatus::Active,
                owner_id: None,
                slug: "p1".into(),
                group_id: None,
            },
        );
        mock.baselines.push(crate::models::Baseline {
            id: 1,
            project_id: 2,
            name: "v1".into(),
            description: None,
            created_at: epoch(),
            created_by: 1,
            source_saved_view_id: None,
            source_view_definition: None,
        });
        let cached = CacheRepository::new(mock, 0);
        let state = AppState::<DieselCachedRepo> {
            repo: Arc::new(RwLock::new(cached)),
        };
        let service = ReqIFService::new(&state);
        let result = service.export_baseline(1, 1);
        assert!(result.is_err());
    }
}
