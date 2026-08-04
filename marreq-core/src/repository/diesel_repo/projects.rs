// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use super::DieselRepo;
use crate::models::entities::*;
use crate::models::forms::*;
use crate::repository::errors::RepoError;
use crate::repository::{
    GroupsRepository, ProjectMembersRepository, ProjectReviewersRepository, ProjectsRepository,
    UserRepository,
};
use crate::schema;
use diesel::pg::upsert::excluded;
use diesel::prelude::*;
use diesel::{Connection, OptionalExtension};

impl ProjectMembersRepository for DieselRepo {
    fn get_members_by_project(&self, project_id: i32) -> Result<Vec<ProjectMember>, RepoError> {
        use crate::schema::project_members::dsl;

        let mut conn = self.get_conn()?;
        dsl::project_members
            .filter(dsl::project_id.eq(project_id))
            .order(dsl::user_id)
            .load::<ProjectMember>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_projects_for_user(&self, user_id: i32) -> Result<Vec<ProjectMember>, RepoError> {
        use crate::schema::project_members::dsl;

        let mut conn = self.get_conn()?;
        dsl::project_members
            .filter(dsl::user_id.eq(user_id))
            .order(dsl::project_id)
            .load::<ProjectMember>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn add_project_member(&mut self, new: &NewProjectMember) -> Result<(), RepoError> {
        use crate::schema::project_members::dsl;

        let mut conn = self.get_conn()?;
        diesel::insert_into(dsl::project_members)
            .values(new)
            .on_conflict((dsl::project_id, dsl::user_id))
            .do_update()
            .set((
                dsl::role.eq(excluded(dsl::role)),
                dsl::updated_at.eq(chrono::Utc::now().naive_utc()),
            ))
            .execute(conn.as_mut())?;
        Ok(())
    }

    fn update_project_member_role(
        &mut self,
        project_id: i32,
        user_id: i32,
        new_role: i32,
    ) -> Result<(), RepoError> {
        use crate::schema::project_members::dsl;

        let mut conn = self.get_conn()?;
        let affected = diesel::update(
            dsl::project_members
                .filter(dsl::project_id.eq(project_id))
                .filter(dsl::user_id.eq(user_id)),
        )
        .set((
            dsl::role.eq(new_role),
            dsl::updated_at.eq(chrono::Utc::now().naive_utc()),
        ))
        .execute(conn.as_mut())?;

        if affected == 0 {
            Err(RepoError::NotFound)
        } else {
            Ok(())
        }
    }

    fn remove_project_member(&mut self, project_id: i32, user_id: i32) -> Result<(), RepoError> {
        use crate::schema::project_members::dsl;

        let mut conn = self.get_conn()?;
        let affected = diesel::delete(
            dsl::project_members
                .filter(dsl::project_id.eq(project_id))
                .filter(dsl::user_id.eq(user_id)),
        )
        .execute(conn.as_mut())?;

        if affected == 0 {
            Err(RepoError::NotFound)
        } else {
            Ok(())
        }
    }
}

impl ProjectReviewersRepository for DieselRepo {
    fn is_project_reviewer(&self, project_id: i32, user_id: i32) -> Result<bool, RepoError> {
        use crate::schema::project_reviewers::dsl::{
            project_id as pr_pid, project_reviewers, user_id as pr_uid,
        };
        let mut conn = self.get_conn()?;
        Ok(project_reviewers
            .filter(pr_pid.eq(project_id))
            .filter(pr_uid.eq(user_id))
            .select(pr_uid)
            .first::<i32>(conn.as_mut())
            .optional()
            .map_err(RepoError::from)?
            .is_some())
    }

    fn list_project_reviewer_ids(&self, project_id: i32) -> Result<Vec<i32>, RepoError> {
        use crate::schema::project_reviewers::dsl::{
            project_id as pr_pid, project_reviewers, user_id as pr_uid,
        };
        let mut conn = self.get_conn()?;
        project_reviewers
            .filter(pr_pid.eq(project_id))
            .order(pr_uid.asc())
            .select(pr_uid)
            .load::<i32>(conn.as_mut())
            .map_err(RepoError::from)
    }

    fn replace_project_reviewers(
        &mut self,
        project_id: i32,
        user_ids: &[i32],
    ) -> Result<(), RepoError> {
        use crate::schema::project_members::dsl as pm;
        use crate::schema::project_reviewers::dsl as pr;
        let mut conn = self.get_conn()?;
        conn.as_mut().transaction::<(), RepoError, _>(|conn| {
            diesel::delete(pr::project_reviewers.filter(pr::project_id.eq(project_id)))
                .execute(conn)?;
            for &uid in user_ids {
                let is_member = pm::project_members
                    .filter(pm::project_id.eq(project_id))
                    .filter(pm::user_id.eq(uid))
                    .select(pm::user_id)
                    .first::<i32>(conn)
                    .optional()
                    .map_err(RepoError::from)?
                    .is_some();
                if !is_member {
                    return Err(RepoError::BadInput(format!(
                        "user {uid} is not a member of project {project_id}"
                    )));
                }
                diesel::insert_into(pr::project_reviewers)
                    .values((pr::project_id.eq(project_id), pr::user_id.eq(uid)))
                    .execute(conn)?;
            }
            Ok(())
        })?;
        Ok(())
    }
}
impl ProjectsRepository for DieselRepo {
    fn get_projects_all(&self) -> Result<Vec<Project>, RepoError> {
        use schema::projects::dsl;
        let mut conn = self.get_conn()?;
        dsl::projects
            .load::<Project>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_project_by_id(&self, project_id: i32) -> Result<Project, RepoError> {
        use schema::projects::dsl;
        let mut conn = self.get_conn()?;
        dsl::projects
            .filter(dsl::id.eq(project_id))
            .first::<Project>(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })
    }

    fn get_project_by_slug(&self, project_slug: &str) -> Result<Project, RepoError> {
        use schema::projects::dsl;
        let mut conn = self.get_conn()?;
        let projects = dsl::projects
            .filter(dsl::slug.eq(project_slug))
            .load::<Project>(conn.as_mut())?;

        match projects.len() {
            0 => Err(RepoError::NotFound),
            1 => Ok(projects.into_iter().next().expect("single project")),
            _ => Err(RepoError::BadInput(format!(
                "project slug '{project_slug}' is ambiguous across namespaces"
            ))),
        }
    }

    fn get_project_by_user_namespace_and_slug(
        &self,
        username: &str,
        slug: &str,
    ) -> Result<Project, RepoError> {
        use schema::projects::dsl;

        let user = self
            .get_user_by_username(username)?
            .ok_or(RepoError::NotFound)?;
        let mut conn = self.get_conn()?;
        dsl::projects
            .filter(dsl::group_id.is_null())
            .filter(dsl::owner_id.eq(Some(user.id)))
            .filter(dsl::slug.eq(slug))
            .first::<Project>(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })
    }

    fn get_project_by_group_namespace_and_slug(
        &self,
        group_slug: &str,
        slug: &str,
    ) -> Result<Project, RepoError> {
        use schema::projects::dsl;

        let group = self.get_group_by_slug(group_slug)?;
        let mut conn = self.get_conn()?;
        dsl::projects
            .filter(dsl::group_id.eq(Some(group.id)))
            .filter(dsl::slug.eq(slug))
            .first::<Project>(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })
    }

    fn insert_new_project(&mut self, new: &NewProjectRow) -> Result<i32, RepoError> {
        use schema::projects::dsl;
        let mut conn = self.get_conn()?;
        let result = diesel::insert_into(dsl::projects)
            .values(new)
            .get_result::<Project>(conn.as_mut())?;
        Ok(result.id)
    }

    fn edit_project(
        &mut self,
        project_id_param: i32,
        update: &UpdateProject,
    ) -> Result<bool, RepoError> {
        use schema::projects::dsl;
        let mut conn = self.get_conn()?;
        let existing = dsl::projects
            .filter(dsl::id.eq(project_id_param))
            .first::<Project>(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })?;
        let slug_value = update.slug.as_deref().unwrap_or(&existing.slug);

        // Build update statement conditionally based on whether status is provided
        let updated = if let Some(status) = update.status {
            diesel::update(dsl::projects.filter(dsl::id.eq(project_id_param)))
                .set((
                    dsl::name.eq(&update.name),
                    dsl::description.eq(&update.description),
                    dsl::status.eq(status),
                    dsl::owner_id.eq(&update.owner_id),
                    dsl::slug.eq(slug_value),
                    dsl::group_id.eq(&update.group_id),
                    dsl::update_date.eq(chrono::Utc::now().naive_utc()),
                ))
                .execute(conn.as_mut())?
        } else {
            diesel::update(dsl::projects.filter(dsl::id.eq(project_id_param)))
                .set((
                    dsl::name.eq(&update.name),
                    dsl::description.eq(&update.description),
                    dsl::owner_id.eq(&update.owner_id),
                    dsl::slug.eq(slug_value),
                    dsl::group_id.eq(&update.group_id),
                    dsl::update_date.eq(chrono::Utc::now().naive_utc()),
                ))
                .execute(conn.as_mut())?
        };
        Ok(updated > 0)
    }

    fn delete_project(&mut self, project_id_param: i32) -> Result<Project, RepoError> {
        use schema::projects::dsl;
        let mut conn = self.get_conn()?;
        let proj = dsl::projects
            .filter(dsl::id.eq(project_id_param))
            .get_result::<Project>(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })?;
        diesel::delete(dsl::projects.filter(dsl::id.eq(project_id_param)))
            .execute(conn.as_mut())?;
        Ok(proj)
    }
}
