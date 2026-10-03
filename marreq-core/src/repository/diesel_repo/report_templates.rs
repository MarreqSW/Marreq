// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Saved report templates (issue #354).

use super::DieselRepo;
use crate::models::{ReportTemplate, ReportTemplateWrite};
use crate::repository::ReportTemplateRepository;
use crate::repository::errors::RepoError;
use crate::schema;
use diesel::prelude::*;
use diesel::result::{DatabaseErrorKind, Error as DieselError};

fn map_write_error(e: DieselError) -> RepoError {
    match e {
        DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => {
            RepoError::Duplicate("you already have a report template with this name".into())
        }
        DieselError::NotFound => RepoError::NotFound,
        e => e.into(),
    }
}

impl ReportTemplateRepository for DieselRepo {
    fn list_report_templates(&self, project_id: i32) -> Result<Vec<ReportTemplate>, RepoError> {
        use schema::report_templates::dsl;
        let mut conn = self.get_conn()?;
        Ok(dsl::report_templates
            .filter(dsl::project_id.eq(project_id))
            .order((dsl::name.asc(), dsl::id.asc()))
            .load(conn.as_mut())?)
    }

    fn get_report_template(&self, id: i32) -> Result<ReportTemplate, RepoError> {
        use schema::report_templates::dsl;
        let mut conn = self.get_conn()?;
        dsl::report_templates
            .find(id)
            .first(conn.as_mut())
            .map_err(map_write_error)
    }

    fn create_report_template(
        &mut self,
        template: &ReportTemplateWrite,
    ) -> Result<ReportTemplate, RepoError> {
        use schema::report_templates::dsl;
        let mut conn = self.get_conn()?;
        diesel::insert_into(dsl::report_templates)
            .values(template)
            .get_result(conn.as_mut())
            .map_err(map_write_error)
    }

    fn update_report_template(
        &mut self,
        id: i32,
        template: &ReportTemplateWrite,
    ) -> Result<ReportTemplate, RepoError> {
        use schema::report_templates::dsl;
        let mut conn = self.get_conn()?;
        diesel::update(dsl::report_templates.find(id))
            .set(template)
            .get_result(conn.as_mut())
            .map_err(map_write_error)
    }

    fn delete_report_template(&mut self, id: i32) -> Result<(), RepoError> {
        use schema::report_templates::dsl;
        let mut conn = self.get_conn()?;
        match diesel::delete(dsl::report_templates.find(id)).execute(conn.as_mut())? {
            0 => Err(RepoError::NotFound),
            _ => Ok(()),
        }
    }
}
