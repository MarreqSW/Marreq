//! Service for ReqIF 1.2 export and import.

use crate::app::{AppState, DieselCachedRepo};
use crate::models::{NewRequirement, User};
use crate::repository::errors::RepoError;
use crate::repository::{RequirementCommentsRepository, UserRepository};
use crate::reqif::import::{object_to_fields, parse_reqif, ImportConfig, ImportResult};
use crate::reqif::to_reqif;
use crate::services::{BaselineService, ProjectService, RequirementService, StatusService};
use std::collections::HashMap;

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
            .filter_map(|r| r.parent_id.map(|p| (r.id, p)))
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
        let parent_map: HashMap<i32, i32> = requirements
            .iter()
            .filter_map(|r| r.parent_id.map(|p| (r.id, p)))
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

        // child (source) -> parent (target)
        let mut parent_of: HashMap<String, String> = HashMap::new();
        for rel in &doc.relations {
            parent_of.insert(rel.source.clone(), rel.target.clone());
        }

        let mut reqif_id_to_reqman_id: HashMap<String, i32> = HashMap::new();
        let mut imported_count = 0usize;
        let mut errors = Vec::new();
        let mut imported_requirement_ids = Vec::new();

        // Pass 1: create all requirements with parent_id = None
        for obj in &doc.objects {
            let (title_opt, ref_opt, desc_opt, status_opt, justification_opt) =
                object_to_fields(obj);

            let title = title_opt.unwrap_or_else(|| "Imported Requirement".to_string());
            let reference_code = ref_opt.unwrap_or_else(|| format!("REQ-{}", obj.id));
            let description = desc_opt.unwrap_or_default();
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
                parent_id: None,
                reference_code: reference_code.clone(),
                reviewer_id: config.reviewer_id,
                applicability_id: config.default_applicability_id,
                justification,
                project_id: config.project_id,
            };

            let verification_method_ids = [config.default_verification_method_id];
            match req_service.create(actor, payload, &verification_method_ids, None) {
                Ok(id) => {
                    imported_count += 1;
                    imported_requirement_ids.push(id);
                    reqif_id_to_reqman_id.insert(obj.id.clone(), id);
                }
                Err(e) => {
                    errors.push(format!("{} ({}): {}", reference_code, title, e));
                }
            }
        }

        // Pass 2: set parent_id for requirements that have a parent in relations
        for obj in &doc.objects {
            let Some(child_reqman_id) = reqif_id_to_reqman_id.get(&obj.id).copied() else {
                continue;
            };
            let Some(parent_reqif_id) = parent_of.get(&obj.id) else {
                continue;
            };
            let Some(parent_reqman_id) = reqif_id_to_reqman_id.get(parent_reqif_id).copied() else {
                continue;
            };
            let req = req_service
                .get_by_id(child_reqman_id)
                .map_err(|e| e.to_string())?;
            let verification_method_ids = req_service
                .get_verification_method_ids(child_reqman_id)
                .unwrap_or_default();
            let payload = NewRequirement {
                id: Some(req.id),
                title: req.title.clone(),
                description: req.description.clone(),
                author_id: req.author_id,
                category_id: req.category_id,
                status_id: req.status_id,
                parent_id: Some(parent_reqman_id),
                reference_code: req.reference_code.clone(),
                reviewer_id: req.reviewer_id,
                applicability_id: req.applicability_id,
                justification: req.justification.clone(),
                project_id: req.project_id,
            };
            let _ = req_service.update(
                actor,
                child_reqman_id,
                payload,
                &verification_method_ids,
                None,
            );
        }

        Ok(ImportResult {
            success: errors.is_empty(),
            message: if errors.is_empty() {
                format!("Successfully imported {} requirements", imported_count)
            } else {
                format!(
                    "Imported {} requirements with {} errors",
                    imported_count,
                    errors.len()
                )
            },
            imported_count,
            errors,
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
            },
        );
        mock.baselines.push(crate::models::Baseline {
            id: 1,
            project_id: 1,
            name: "v1.0".to_string(),
            description: None,
            created_at: epoch(),
            created_by: 1,
        });
        mock.requirement_statuses.insert(
            1,
            crate::models::RequirementStatus {
                id: 1,
                title: "Draft".into(),
                description: "".into(),
                tag: "D".into(),
                project_id: 1,
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
                parent_id: None,
                applicability_id: 1,
                justification: None,
                deadline_date: None,
                created_at: epoch(),
                approval_state: "draft".into(),
                approved_by: None,
                approved_at: None,
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
            },
        );
        mock.baselines.push(crate::models::Baseline {
            id: 1,
            project_id: 2,
            name: "v1".into(),
            description: None,
            created_at: epoch(),
            created_by: 1,
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
