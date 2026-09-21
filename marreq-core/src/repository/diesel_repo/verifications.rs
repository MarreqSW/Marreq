// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use super::requirements::requirement_from_current;
use super::{map_db_error, DieselRepo};
use crate::models::entities::*;
use crate::models::forms::*;
use crate::repository::errors::RepoError;
use crate::repository::VerificationsRepository;
use crate::schema;
use diesel::expression_methods::NullableExpressionMethods;
use diesel::prelude::*;
use diesel::{Connection, JoinOnDsl, SelectableHelper};

impl VerificationsRepository for DieselRepo {
    fn get_verification_by_id(&self, verification_id: i32) -> Result<Verification, RepoError> {
        use schema::verifications::dsl;
        let mut conn = self.get_conn()?;
        dsl::verifications
            .filter(dsl::id.eq(verification_id))
            .select(Verification::as_select())
            .get_result(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })
    }

    fn get_verifications_all(&self) -> Result<Vec<Verification>, RepoError> {
        use schema::verifications::dsl;
        let mut conn = self.get_conn()?;
        dsl::verifications
            .order(dsl::id)
            .select(Verification::as_select())
            .load::<Verification>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_verifications_by_project(&self, project: i32) -> Result<Vec<Verification>, RepoError> {
        use schema::verifications::dsl;
        let mut conn = self.get_conn()?;
        dsl::verifications
            .filter(dsl::project_id.eq(project))
            .select(Verification::as_select())
            .load::<Verification>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_verifications_for_requirement(
        &self,
        requirement_id: i32,
    ) -> Result<Vec<Verification>, RepoError> {
        use schema::matrix::dsl;
        use schema::verifications::dsl as v;
        let mut conn = self.get_conn()?;
        dsl::matrix
            .filter(dsl::req_id.eq(requirement_id))
            .inner_join(v::verifications.on(dsl::verification_id.eq(v::id)))
            .select((
                v::id,
                v::name,
                v::reference_code,
                v::description,
                v::source,
                v::status_id,
                v::parent_id,
                v::project_id,
                v::verification_method_id,
                v::author_id,
                v::reviewer_id,
                v::status_set_by,
                v::status_set_at,
            ))
            .load::<Verification>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_impacted_verifications_for_requirement(
        &self,
        requirement_id: i32,
    ) -> Result<Vec<Verification>, RepoError> {
        use schema::matrix::dsl;
        use schema::verifications::dsl as v;
        let mut conn = self.get_conn()?;
        dsl::matrix
            .filter(dsl::req_id.eq(requirement_id))
            .filter(dsl::suspect.eq(true))
            .inner_join(v::verifications.on(dsl::verification_id.eq(v::id)))
            .select((
                v::id,
                v::name,
                v::reference_code,
                v::description,
                v::source,
                v::status_id,
                v::parent_id,
                v::project_id,
                v::verification_method_id,
                v::author_id,
                v::reviewer_id,
                v::status_set_by,
                v::status_set_at,
            ))
            .load::<Verification>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_requirements_for_verification(
        &self,
        verification_id: i32,
    ) -> Result<Vec<Requirement>, RepoError> {
        use schema::matrix;
        use schema::requirement_versions;
        use schema::requirements;
        let mut conn = self.get_conn()?;
        let rows: Vec<(RequirementContainer, RequirementVersion)> = matrix::table
            .filter(matrix::verification_id.eq(verification_id))
            .inner_join(requirements::table.on(matrix::req_id.eq(requirements::id)))
            .inner_join(
                requirement_versions::table
                    .on(requirements::current_version_id.eq(requirement_versions::id.nullable())),
            )
            .select((
                RequirementContainer::as_select(),
                RequirementVersion::as_select(),
            ))
            .load(conn.as_mut())
            .map_err(RepoError::from)?;
        Ok(rows
            .into_iter()
            .map(|(c, v)| requirement_from_current(&c, &v))
            .collect())
    }

    fn insert_verification(&mut self, new: &NewVerification) -> Result<i32, RepoError> {
        self.insert_verification_idempotent(new, None)
    }

    fn insert_verification_idempotent(
        &mut self,
        new: &NewVerification,
        mcp_idempotency_identity: Option<&str>,
    ) -> Result<i32, RepoError> {
        let mut conn = self.get_conn()?;
        let res: Verification = diesel::insert_into(schema::verifications::table)
            .values((
                new,
                schema::verifications::mcp_idempotency_identity.eq(mcp_idempotency_identity),
            ))
            .returning(Verification::as_returning())
            .get_result(conn.as_mut())?;
        Ok(res.id)
    }

    fn edit_verification(&mut self, new: &NewVerification) -> Result<bool, RepoError> {
        use crate::schema::verifications::dsl;
        let mut conn = self.get_conn()?;
        let verification_id_value = new
            .id
            .ok_or(RepoError::Db(diesel::result::Error::NotFound))?;
        let updated = diesel::update(dsl::verifications.filter(dsl::id.eq(verification_id_value)))
            .set((
                dsl::name.eq(&new.name),
                dsl::description.eq(&new.description),
                dsl::source.eq(&new.source),
                dsl::reference_code.eq(&new.reference_code),
                dsl::status_id.eq(&new.status_id),
                dsl::parent_id.eq(&new.parent_id),
                dsl::verification_method_id.eq(&new.verification_method_id),
                dsl::author_id.eq(new.author_id),
                dsl::reviewer_id.eq(new.reviewer_id),
            ))
            .execute(conn.as_mut())?;
        Ok(updated > 0)
    }

    fn record_verification_status_audit(
        &mut self,
        verification_id: i32,
        actor_id: i32,
    ) -> Result<(), RepoError> {
        use crate::schema::verifications::dsl;
        let mut conn = self.get_conn()?;
        let now = chrono::Utc::now().naive_utc();
        diesel::update(dsl::verifications.filter(dsl::id.eq(verification_id)))
            .set((
                dsl::status_set_by.eq(Some(actor_id)),
                dsl::status_set_at.eq(Some(now)),
            ))
            .execute(conn.as_mut())?;
        Ok(())
    }

    fn delete_verification(&mut self, verification_id: i32) -> Result<Verification, RepoError> {
        use crate::schema::verifications::dsl;
        let mut conn = self.get_conn()?;
        let verification = dsl::verifications
            .filter(dsl::id.eq(verification_id))
            .select(Verification::as_select())
            .get_result::<Verification>(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })?;
        diesel::delete(dsl::verifications.filter(dsl::id.eq(verification_id)))
            .execute(conn.as_mut())?;
        Ok(verification)
    }

    fn update_verification_requirement_links(
        &mut self,
        verification_id: i32,
        requirement_ids: &[i32],
    ) -> Result<(), RepoError> {
        use schema::matrix::dsl;
        let mut conn = self.get_conn()?;

        conn.as_mut()
            .transaction::<_, diesel::result::Error, _>(|conn| {
                diesel::delete(dsl::matrix.filter(dsl::verification_id.eq(verification_id)))
                    .execute(conn)?;

                for requirement_id in requirement_ids {
                    use crate::schema::verifications::dsl::verifications;
                    use crate::schema::verifications::dsl::{
                        id as verification_id_col, project_id as v_pid,
                    };
                    let project_id: i32 = verifications
                        .filter(verification_id_col.eq(verification_id))
                        .select(v_pid)
                        .first(conn)?;

                    let new_matrix = NewMatrixLink {
                        req_id: *requirement_id,
                        verification_id,
                        project_id,
                        triggering_version_id: None,
                        triggering_user_id: None,
                    };
                    diesel::insert_into(schema::matrix::table)
                        .values(&new_matrix)
                        .execute(conn)?;
                }
                Ok(())
            })
            .map_err(map_db_error)?;
        Ok(())
    }
}
