// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use super::{map_db_error, DieselRepo};
use crate::models::entities::*;
use crate::repository::errors::RepoError;
use crate::repository::RequirementVersionLinksRepository;
use crate::schema;
use diesel::prelude::*;

impl RequirementVersionLinksRepository for DieselRepo {
    fn list_links_by_source_version(
        &self,
        source_version_id: i32,
    ) -> Result<Vec<RequirementVersionLink>, RepoError> {
        use schema::requirement_version_links::dsl;
        let mut conn = self.get_conn()?;
        dsl::requirement_version_links
            .filter(dsl::source_version_id.eq(source_version_id))
            .order(dsl::created_at.asc())
            .load(conn.as_mut())
            .map_err(RepoError::from)
    }

    fn list_links_by_target_version(
        &self,
        target_version_id: i32,
    ) -> Result<Vec<RequirementVersionLink>, RepoError> {
        use schema::requirement_version_links::dsl;
        let mut conn = self.get_conn()?;
        dsl::requirement_version_links
            .filter(dsl::target_version_id.eq(target_version_id))
            .order(dsl::created_at.asc())
            .load(conn.as_mut())
            .map_err(RepoError::from)
    }

    fn list_links_by_project(
        &self,
        project_id: i32,
        source_version_id: Option<i32>,
        target_version_id: Option<i32>,
        link_type: Option<&str>,
    ) -> Result<Vec<RequirementVersionLink>, RepoError> {
        use schema::requirement_version_links::dsl;
        let mut conn = self.get_conn()?;
        let mut q = dsl::requirement_version_links
            .filter(dsl::project_id.eq(project_id))
            .into_boxed();
        if let Some(sid) = source_version_id {
            q = q.filter(dsl::source_version_id.eq(sid));
        }
        if let Some(tid) = target_version_id {
            q = q.filter(dsl::target_version_id.eq(tid));
        }
        if let Some(lt) = link_type {
            q = q.filter(dsl::link_type.eq(lt));
        }
        q.order(dsl::created_at.asc())
            .load(conn.as_mut())
            .map_err(RepoError::from)
    }

    fn insert_requirement_version_link(
        &mut self,
        new: &NewRequirementVersionLink,
    ) -> Result<RequirementVersionLink, RepoError> {
        let mut conn = self.get_conn()?;
        diesel::insert_into(schema::requirement_version_links::table)
            .values(new)
            .returning(schema::requirement_version_links::all_columns)
            .get_result(conn.as_mut())
            .map_err(map_db_error)
    }

    fn delete_requirement_version_link(
        &mut self,
        link_id: i32,
    ) -> Result<RequirementVersionLink, RepoError> {
        use schema::requirement_version_links::dsl as rvl_dsl;
        let mut conn = self.get_conn()?;
        let link = rvl_dsl::requirement_version_links
            .filter(rvl_dsl::id.eq(link_id))
            .get_result::<RequirementVersionLink>(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })?;
        diesel::delete(rvl_dsl::requirement_version_links.filter(rvl_dsl::id.eq(link_id)))
            .execute(conn.as_mut())?;
        Ok(link)
    }

    fn delete_requirement_version_links_by_source_version(
        &mut self,
        source_version_id: i32,
    ) -> Result<Vec<RequirementVersionLink>, RepoError> {
        use schema::requirement_version_links::dsl as rvl_dsl;
        let mut conn = self.get_conn()?;
        let links: Vec<RequirementVersionLink> = rvl_dsl::requirement_version_links
            .filter(rvl_dsl::source_version_id.eq(source_version_id))
            .load(conn.as_mut())?;
        let ids: Vec<i32> = links.iter().map(|l| l.id).collect();
        for id in ids {
            diesel::delete(rvl_dsl::requirement_version_links.filter(rvl_dsl::id.eq(id)))
                .execute(conn.as_mut())?;
        }
        Ok(links)
    }

    fn get_requirement_version_link_by_id(
        &self,
        link_id: i32,
    ) -> Result<RequirementVersionLink, RepoError> {
        use schema::requirement_version_links::dsl as rvl_dsl;
        let mut conn = self.get_conn()?;
        rvl_dsl::requirement_version_links
            .filter(rvl_dsl::id.eq(link_id))
            .get_result(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })
    }
}
