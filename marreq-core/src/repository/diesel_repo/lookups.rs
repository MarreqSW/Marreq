// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use super::DieselRepo;
use crate::models::entities::*;
use crate::models::forms::*;
use crate::repository::errors::RepoError;
use crate::repository::LookupRepository;
use crate::schema;
use diesel::prelude::*;

impl LookupRepository for DieselRepo {
    fn get_requirement_status_all(&self) -> Result<Vec<RequirementStatus>, RepoError> {
        use schema::requirement_status::dsl;
        let mut conn = self.get_conn()?;
        dsl::requirement_status
            .order(dsl::id)
            .load::<RequirementStatus>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_requirement_status_by_project(
        &self,
        project_id: i32,
    ) -> Result<Vec<RequirementStatus>, RepoError> {
        use schema::requirement_status::dsl;
        let mut conn = self.get_conn()?;
        dsl::requirement_status
            .filter(dsl::project_id.eq(project_id))
            .order(dsl::id)
            .load::<RequirementStatus>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_requirement_status_by_id(&self, status_id: i32) -> Result<RequirementStatus, RepoError> {
        use schema::requirement_status::dsl;
        let mut conn = self.get_conn()?;
        dsl::requirement_status
            .filter(dsl::id.eq(status_id))
            .get_result(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })
    }

    fn get_verification_status_all(&self) -> Result<Vec<VerificationStatus>, RepoError> {
        use schema::verification_status::dsl;
        let mut conn = self.get_conn()?;
        dsl::verification_status
            .order(dsl::id)
            .load::<VerificationStatus>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_verification_status_by_project(
        &self,
        project_id: i32,
    ) -> Result<Vec<VerificationStatus>, RepoError> {
        use schema::verification_status::dsl;
        let mut conn = self.get_conn()?;
        dsl::verification_status
            .filter(dsl::project_id.eq(project_id))
            .order(dsl::id)
            .load::<VerificationStatus>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_verification_status_by_id(
        &self,
        status_id: i32,
    ) -> Result<VerificationStatus, RepoError> {
        use schema::verification_status::dsl;
        let mut conn = self.get_conn()?;
        dsl::verification_status
            .filter(dsl::id.eq(status_id))
            .get_result(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })
    }

    fn get_categories_all(&self) -> Result<Vec<Category>, RepoError> {
        use schema::categories::dsl;
        let mut conn = self.get_conn()?;
        dsl::categories
            .order(dsl::id)
            .load::<Category>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_category_by_id(&self, category_id: i32) -> Result<Category, RepoError> {
        use schema::categories::dsl;
        let mut conn = self.get_conn()?;
        dsl::categories
            .filter(dsl::id.eq(category_id))
            .get_result(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })
    }

    fn get_categories_by_project(&self, project_id: i32) -> Result<Vec<Category>, RepoError> {
        use schema::categories::dsl;
        let mut conn = self.get_conn()?;
        dsl::categories
            .filter(dsl::project_id.eq(project_id))
            .load::<Category>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_applicability_all(&self) -> Result<Vec<Applicability>, RepoError> {
        use schema::applicability::dsl;
        let mut conn = self.get_conn()?;
        dsl::applicability
            .order(dsl::id)
            .load::<Applicability>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_applicability_by_id(&self, applicability_id: i32) -> Result<Applicability, RepoError> {
        use schema::applicability::dsl;
        let mut conn = self.get_conn()?;
        dsl::applicability
            .filter(dsl::id.eq(applicability_id))
            .get_result(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })
    }

    fn get_applicability_by_project(
        &self,
        project_id: i32,
    ) -> Result<Vec<Applicability>, RepoError> {
        use schema::applicability::dsl;
        let mut conn = self.get_conn()?;
        dsl::applicability
            .filter(dsl::project_id.eq(project_id))
            .load::<Applicability>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_verification_methods_all(&self) -> Result<Vec<VerificationMethod>, RepoError> {
        use schema::verification_methods::dsl;
        let mut conn = self.get_conn()?;
        dsl::verification_methods
            .order(dsl::id)
            .load::<VerificationMethod>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_verification_method_by_id(
        &self,
        verification_method_id: i32,
    ) -> Result<VerificationMethod, RepoError> {
        use schema::verification_methods::dsl;
        let mut conn = self.get_conn()?;
        dsl::verification_methods
            .filter(dsl::id.eq(verification_method_id))
            .get_result(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })
    }

    fn get_verification_methods_by_project(
        &self,
        project_id: i32,
    ) -> Result<Vec<VerificationMethod>, RepoError> {
        use schema::verification_methods::dsl;
        let mut conn = self.get_conn()?;
        dsl::verification_methods
            .filter(dsl::project_id.eq(project_id))
            .order(dsl::id)
            .load::<VerificationMethod>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn insert_new_verification_method(
        &mut self,
        new: &NewVerificationMethod,
    ) -> Result<i32, RepoError> {
        let mut conn = self.get_conn()?;
        let result = diesel::insert_into(schema::verification_methods::table)
            .values(new)
            .get_result::<VerificationMethod>(conn.as_mut())?;
        Ok(result.id)
    }

    fn edit_verification_method(&mut self, new: &NewVerificationMethod) -> Result<bool, RepoError> {
        use schema::verification_methods::dsl;
        let mut conn = self.get_conn()?;
        let verification_method_id = new
            .id
            .ok_or(RepoError::Db(diesel::result::Error::NotFound))?;
        let updated =
            diesel::update(dsl::verification_methods.filter(dsl::id.eq(verification_method_id)))
                .set((
                    dsl::title.eq(&new.title),
                    dsl::description.eq(&new.description),
                    dsl::tag.eq(&new.tag),
                ))
                .execute(conn.as_mut())?;
        Ok(updated > 0)
    }

    fn delete_verification_method(
        &mut self,
        verification_method_id: i32,
    ) -> Result<VerificationMethod, RepoError> {
        use schema::verification_methods::dsl;
        let mut conn = self.get_conn()?;
        let verification = dsl::verification_methods
            .filter(dsl::id.eq(verification_method_id))
            .get_result::<VerificationMethod>(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })?;
        diesel::delete(dsl::verification_methods.filter(dsl::id.eq(verification_method_id)))
            .execute(conn.as_mut())?;
        Ok(verification)
    }

    fn insert_new_category(&mut self, new: &NewCategory) -> Result<i32, RepoError> {
        use schema::categories::dsl;
        let mut conn = self.get_conn()?;
        let result = diesel::insert_into(dsl::categories)
            .values(new)
            .get_result::<Category>(conn.as_mut())?;
        Ok(result.id)
    }

    fn edit_category(&mut self, new: &NewCategory) -> Result<bool, RepoError> {
        use schema::categories::dsl;
        let mut conn = self.get_conn()?;
        let category_id = new
            .id
            .ok_or(RepoError::Db(diesel::result::Error::NotFound))?;
        let updated = diesel::update(dsl::categories.filter(dsl::id.eq(category_id)))
            .set((
                dsl::title.eq(&new.title),
                dsl::description.eq(&new.description),
                dsl::tag.eq(&new.tag),
            ))
            .execute(conn.as_mut())?;
        Ok(updated > 0)
    }

    fn delete_category(&mut self, category_id: i32) -> Result<Category, RepoError> {
        use schema::categories::dsl;
        let mut conn = self.get_conn()?;
        let cat = dsl::categories
            .filter(dsl::id.eq(category_id))
            .get_result::<Category>(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })?;
        diesel::delete(dsl::categories.filter(dsl::id.eq(category_id))).execute(conn.as_mut())?;
        Ok(cat)
    }

    fn insert_new_applicability(&mut self, new: &NewApplicability) -> Result<i32, RepoError> {
        use schema::applicability::dsl;
        let mut conn = self.get_conn()?;
        let result = diesel::insert_into(dsl::applicability)
            .values(new)
            .get_result::<Applicability>(conn.as_mut())?;
        Ok(result.id)
    }

    fn edit_applicability(&mut self, new: &NewApplicability) -> Result<bool, RepoError> {
        use schema::applicability::dsl;
        let mut conn = self.get_conn()?;
        let app_id_val = new
            .id
            .ok_or(RepoError::Db(diesel::result::Error::NotFound))?;
        let updated = diesel::update(dsl::applicability.filter(dsl::id.eq(app_id_val)))
            .set((
                dsl::title.eq(&new.title),
                dsl::description.eq(&new.description),
                dsl::tag.eq(&new.tag),
            ))
            .execute(conn.as_mut())?;
        Ok(updated > 0)
    }

    fn delete_applicability(&mut self, applicability_id: i32) -> Result<Applicability, RepoError> {
        use schema::applicability::dsl;
        let mut conn = self.get_conn()?;
        let app = dsl::applicability
            .filter(dsl::id.eq(applicability_id))
            .get_result::<Applicability>(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })?;
        diesel::delete(dsl::applicability.filter(dsl::id.eq(applicability_id)))
            .execute(conn.as_mut())?;
        Ok(app)
    }

    fn create_requirement_status(&mut self, new: &NewRequirementStatus) -> Result<i32, RepoError> {
        let mut conn = self.get_conn()?;
        let res: RequirementStatus = diesel::insert_into(schema::requirement_status::table)
            .values(new)
            .get_result(conn.as_mut())?;
        Ok(res.id)
    }

    fn create_verification_status(
        &mut self,
        new: &NewVerificationStatus,
    ) -> Result<i32, RepoError> {
        let mut conn = self.get_conn()?;
        let res: VerificationStatus = diesel::insert_into(schema::verification_status::table)
            .values(new)
            .get_result(conn.as_mut())?;
        Ok(res.id)
    }

    fn update_requirement_status(
        &mut self,
        id: i32,
        payload: &NewRequirementStatus,
    ) -> Result<bool, RepoError> {
        use schema::requirement_status::dsl;
        let status = self.get_requirement_status_by_id(id)?;
        if status.is_system {
            return Err(RepoError::BadInput("Cannot modify system status".into()));
        }
        let mut conn = self.get_conn()?;
        let updated = diesel::update(dsl::requirement_status.filter(dsl::id.eq(id)))
            .set((
                dsl::title.eq(&payload.title),
                dsl::description.eq(&payload.description),
                dsl::tag.eq(&payload.tag),
                dsl::tag_color.eq(&payload.tag_color),
            ))
            .execute(conn.as_mut())?;
        Ok(updated > 0)
    }

    fn delete_requirement_status(&mut self, id: i32) -> Result<RequirementStatus, RepoError> {
        use schema::{requirement_status::dsl, requirement_versions};
        let status = self.get_requirement_status_by_id(id)?;
        if status.is_system {
            return Err(RepoError::BadInput("Cannot delete system status".into()));
        }
        let mut conn = self.get_conn()?;
        let in_use: i64 = requirement_versions::table
            .filter(requirement_versions::status_id.eq(id))
            .count()
            .get_result(conn.as_mut())
            .map_err(RepoError::from)?;
        if in_use > 0 {
            return Err(RepoError::BadInput(
                "Cannot delete status: it is in use by requirement versions".into(),
            ));
        }
        diesel::delete(dsl::requirement_status.filter(dsl::id.eq(id))).execute(conn.as_mut())?;
        Ok(status)
    }

    fn update_verification_status(
        &mut self,
        id: i32,
        payload: &NewVerificationStatus,
    ) -> Result<bool, RepoError> {
        use schema::verification_status::dsl;
        let status = self.get_verification_status_by_id(id)?;
        if status.is_system {
            return Err(RepoError::BadInput("Cannot modify system status".into()));
        }
        let mut conn = self.get_conn()?;
        let updated = diesel::update(dsl::verification_status.filter(dsl::id.eq(id)))
            .set((
                dsl::title.eq(&payload.title),
                dsl::description.eq(&payload.description),
                dsl::tag.eq(&payload.tag),
                dsl::tag_color.eq(&payload.tag_color),
            ))
            .execute(conn.as_mut())?;
        Ok(updated > 0)
    }

    fn delete_verification_status(&mut self, id: i32) -> Result<VerificationStatus, RepoError> {
        use schema::{verification_status::dsl, verifications};
        let status = self.get_verification_status_by_id(id)?;
        if status.is_system {
            return Err(RepoError::BadInput("Cannot delete system status".into()));
        }
        let mut conn = self.get_conn()?;
        let in_use: i64 = verifications::table
            .filter(verifications::status_id.eq(id))
            .count()
            .get_result(conn.as_mut())
            .map_err(RepoError::from)?;
        if in_use > 0 {
            return Err(RepoError::BadInput(
                "Cannot delete status: it is in use by verifications".into(),
            ));
        }
        diesel::delete(dsl::verification_status.filter(dsl::id.eq(id))).execute(conn.as_mut())?;
        Ok(status)
    }
}
