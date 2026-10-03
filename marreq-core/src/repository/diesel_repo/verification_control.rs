// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Verification control data (issue #353).

use super::DieselRepo;
use crate::models::{RequirementCompliance, VerificationControl};
use crate::repository::VerificationControlRepository;
use crate::repository::errors::RepoError;
use crate::schema;
use diesel::prelude::*;

impl VerificationControlRepository for DieselRepo {
    fn get_verification_control(
        &self,
        verification_id: i32,
    ) -> Result<Option<VerificationControl>, RepoError> {
        use schema::verification_control::dsl;
        let mut conn = self.get_conn()?;
        Ok(dsl::verification_control
            .find(verification_id)
            .first(conn.as_mut())
            .optional()?)
    }

    fn list_verification_control_by_project(
        &self,
        project_id: i32,
    ) -> Result<Vec<VerificationControl>, RepoError> {
        use schema::verification_control::dsl;
        let mut conn = self.get_conn()?;
        Ok(dsl::verification_control
            .filter(dsl::project_id.eq(project_id))
            .order(dsl::verification_id)
            .load(conn.as_mut())?)
    }

    fn upsert_verification_control(
        &mut self,
        control: &VerificationControl,
    ) -> Result<VerificationControl, RepoError> {
        use schema::verification_control::dsl;
        let mut conn = self.get_conn()?;
        Ok(diesel::insert_into(dsl::verification_control)
            .values(control)
            .on_conflict(dsl::verification_id)
            .do_update()
            .set(control)
            .get_result(conn.as_mut())?)
    }

    fn get_requirement_compliance(
        &self,
        requirement_id: i32,
    ) -> Result<Option<RequirementCompliance>, RepoError> {
        use schema::requirement_compliance::dsl;
        let mut conn = self.get_conn()?;
        Ok(dsl::requirement_compliance
            .find(requirement_id)
            .first(conn.as_mut())
            .optional()?)
    }

    fn list_requirement_compliance_by_project(
        &self,
        project_id: i32,
    ) -> Result<Vec<RequirementCompliance>, RepoError> {
        use schema::requirement_compliance::dsl;
        let mut conn = self.get_conn()?;
        Ok(dsl::requirement_compliance
            .filter(dsl::project_id.eq(project_id))
            .order(dsl::requirement_id)
            .load(conn.as_mut())?)
    }

    fn set_requirement_compliance(
        &mut self,
        compliance: &RequirementCompliance,
    ) -> Result<RequirementCompliance, RepoError> {
        use schema::requirement_compliance::dsl;
        let mut conn = self.get_conn()?;
        Ok(diesel::insert_into(dsl::requirement_compliance)
            .values(compliance)
            .on_conflict(dsl::requirement_id)
            .do_update()
            .set(compliance)
            .get_result(conn.as_mut())?)
    }

    fn clear_requirement_compliance(&mut self, requirement_id: i32) -> Result<bool, RepoError> {
        use schema::requirement_compliance::dsl;
        let mut conn = self.get_conn()?;
        let n = diesel::delete(dsl::requirement_compliance.find(requirement_id))
            .execute(conn.as_mut())?;
        Ok(n > 0)
    }
}
