// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use super::{map_db_error, map_unique_violation, DieselRepo};
use crate::models::entities::*;
use crate::models::forms::*;
use crate::repository::errors::RepoError;
use crate::repository::{GroupMembersRepository, GroupsRepository};
use crate::schema;
use diesel::pg::upsert::excluded;
use diesel::prelude::*;

impl GroupsRepository for DieselRepo {
    fn get_groups_all(&self) -> Result<Vec<Group>, RepoError> {
        use schema::groups::dsl;
        let mut conn = self.get_conn()?;
        dsl::groups
            .load::<Group>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_group_by_id(&self, group_id: i32) -> Result<Group, RepoError> {
        use schema::groups::dsl;
        let mut conn = self.get_conn()?;
        dsl::groups
            .filter(dsl::id.eq(group_id))
            .first::<Group>(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })
    }

    fn get_group_by_slug(&self, group_slug: &str) -> Result<Group, RepoError> {
        use schema::groups::dsl;
        let mut conn = self.get_conn()?;
        dsl::groups
            .filter(dsl::slug.eq(group_slug))
            .first::<Group>(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })
    }

    fn insert_new_group(&mut self, new: &NewGroupRow) -> Result<i32, RepoError> {
        use schema::groups::dsl;
        let mut conn = self.get_conn()?;
        let result = diesel::insert_into(dsl::groups)
            .values(new)
            .get_result::<Group>(conn.as_mut())
            .map_err(map_unique_violation)?;
        Ok(result.id)
    }

    fn edit_group(&mut self, group_id_param: i32, update: &UpdateGroup) -> Result<bool, RepoError> {
        use schema::groups::dsl;
        let mut conn = self.get_conn()?;
        let updated = diesel::update(dsl::groups.filter(dsl::id.eq(group_id_param)))
            .set((
                dsl::name.eq(&update.name),
                dsl::description.eq(&update.description),
                dsl::owner_id.eq(&update.owner_id),
                dsl::updated_at.eq(chrono::Utc::now().naive_utc()),
            ))
            .execute(conn.as_mut())?;
        Ok(updated > 0)
    }

    fn delete_group(&mut self, group_id_param: i32) -> Result<Group, RepoError> {
        use schema::groups::dsl;
        let mut conn = self.get_conn()?;
        let group = dsl::groups
            .filter(dsl::id.eq(group_id_param))
            .get_result::<Group>(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })?;
        diesel::delete(dsl::groups.filter(dsl::id.eq(group_id_param))).execute(conn.as_mut())?;
        Ok(group)
    }

    fn get_projects_by_group(&self, group_id: i32) -> Result<Vec<Project>, RepoError> {
        use schema::projects::dsl;
        let mut conn = self.get_conn()?;
        dsl::projects
            .filter(dsl::group_id.eq(group_id))
            .load::<Project>(conn.as_mut())
            .map_err(|e| e.into())
    }
}

impl GroupMembersRepository for DieselRepo {
    fn get_members_by_group(&self, gid: i32) -> Result<Vec<GroupMember>, RepoError> {
        use schema::group_members::dsl;
        let mut conn = self.get_conn()?;
        dsl::group_members
            .filter(dsl::group_id.eq(gid))
            .load::<GroupMember>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_groups_for_user(&self, uid: i32) -> Result<Vec<GroupMember>, RepoError> {
        use schema::group_members::dsl;
        let mut conn = self.get_conn()?;
        dsl::group_members
            .filter(dsl::user_id.eq(uid))
            .load::<GroupMember>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn add_group_member(&mut self, new: &NewGroupMember) -> Result<(), RepoError> {
        let mut conn = self.get_conn()?;
        diesel::insert_into(schema::group_members::table)
            .values(new)
            .on_conflict((
                schema::group_members::group_id,
                schema::group_members::user_id,
            ))
            .do_update()
            .set(schema::group_members::role.eq(excluded(schema::group_members::role)))
            .execute(conn.as_mut())
            .map_err(map_db_error)?;
        Ok(())
    }

    fn update_group_member_role(
        &mut self,
        gid: i32,
        uid: i32,
        new_role: i32,
    ) -> Result<(), RepoError> {
        use schema::group_members::dsl;
        let mut conn = self.get_conn()?;
        let updated = diesel::update(
            dsl::group_members
                .filter(dsl::group_id.eq(gid))
                .filter(dsl::user_id.eq(uid)),
        )
        .set((
            dsl::role.eq(new_role),
            dsl::updated_at.eq(chrono::Utc::now().naive_utc()),
        ))
        .execute(conn.as_mut())?;
        if updated == 0 {
            Err(RepoError::NotFound)
        } else {
            Ok(())
        }
    }

    fn remove_group_member(&mut self, gid: i32, uid: i32) -> Result<(), RepoError> {
        use schema::group_members::dsl;
        let mut conn = self.get_conn()?;
        let deleted = diesel::delete(
            dsl::group_members
                .filter(dsl::group_id.eq(gid))
                .filter(dsl::user_id.eq(uid)),
        )
        .execute(conn.as_mut())?;
        if deleted == 0 {
            Err(RepoError::NotFound)
        } else {
            Ok(())
        }
    }
}
