//! Service for ReqIF 1.2 export and import.

use crate::app::{AppState, DieselCachedRepo};
use crate::models::{NewRequirement, User};
use crate::repository::errors::RepoError;
use crate::reqif::import::{object_to_fields, parse_reqif, ImportConfig, ImportResult};
use crate::reqif::to_reqif;
use crate::services::{ProjectService, RequirementService, StatusService};
use std::collections::HashMap;

pub struct ReqIFService<'a> {
    state: &'a AppState<DieselCachedRepo>,
}

impl<'a> ReqIFService<'a> {
    pub fn new(state: &'a AppState<DieselCachedRepo>) -> Self {
        Self { state }
    }

    /// Export project requirements as ReqIF 1.2 XML.
    pub fn export_project(&self, project_id: i32) -> Result<String, RepoError> {
        let req_service = RequirementService::new(self.state);
        let project_service = ProjectService::new(self.state);
        let project = project_service.get_by_id(project_id)?;
        let requirements = req_service.list_by_project(project_id)?;
        let mut parent_map = HashMap::new();
        for r in &requirements {
            if let Some(p) = r.parent_id {
                parent_map.insert(r.id, p);
            }
        }
        Ok(to_reqif(&project.name, &requirements, &parent_map))
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
            match req_service.create(actor, payload, &verification_method_ids) {
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
            let _ = req_service.update(actor, child_reqman_id, payload, &verification_method_ids);
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
