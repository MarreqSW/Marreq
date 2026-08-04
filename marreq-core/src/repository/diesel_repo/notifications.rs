// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use super::DieselRepo;
use crate::models::entities::*;
use crate::models::forms::*;
use crate::repository::errors::RepoError;
use crate::repository::NotificationRepository;
use crate::schema;
use diesel::prelude::*;

impl NotificationRepository for DieselRepo {
    fn insert_notification(&mut self, new: &NewNotification) -> Result<i32, RepoError> {
        use schema::notifications::dsl;
        let mut conn = self.get_conn()?;
        diesel::insert_into(dsl::notifications)
            .values(new)
            .returning(dsl::id)
            .get_result(conn.as_mut())
            .map_err(RepoError::from)
    }

    fn get_notifications_for_user(
        &self,
        user_id: i32,
        limit: i64,
        unread_only: bool,
    ) -> Result<Vec<Notification>, RepoError> {
        use schema::notifications::dsl;
        let mut conn = self.get_conn()?;
        let mut query = dsl::notifications
            .filter(dsl::user_id.eq(user_id))
            .order((dsl::read.asc(), dsl::created_at.desc()))
            .limit(limit)
            .into_boxed();
        if unread_only {
            query = query.filter(dsl::read.eq(false));
        }
        query
            .load::<Notification>(conn.as_mut())
            .map_err(RepoError::from)
    }

    fn count_unread_notifications(&self, user_id: i32) -> Result<i64, RepoError> {
        use schema::notifications::dsl;
        let mut conn = self.get_conn()?;
        dsl::notifications
            .filter(dsl::user_id.eq(user_id))
            .filter(dsl::read.eq(false))
            .count()
            .get_result(conn.as_mut())
            .map_err(RepoError::from)
    }

    fn mark_notification_read(&mut self, id: i32, user_id: i32) -> Result<bool, RepoError> {
        use schema::notifications::dsl;
        let mut conn = self.get_conn()?;
        let count = diesel::update(
            dsl::notifications
                .filter(dsl::id.eq(id))
                .filter(dsl::user_id.eq(user_id)),
        )
        .set(dsl::read.eq(true))
        .execute(conn.as_mut())
        .map_err(RepoError::from)?;
        Ok(count > 0)
    }

    fn mark_all_read(&mut self, user_id: i32) -> Result<usize, RepoError> {
        use schema::notifications::dsl;
        let mut conn = self.get_conn()?;
        diesel::update(
            dsl::notifications
                .filter(dsl::user_id.eq(user_id))
                .filter(dsl::read.eq(false)),
        )
        .set(dsl::read.eq(true))
        .execute(conn.as_mut())
        .map_err(RepoError::from)
    }

    fn get_notification_preferences(
        &self,
        user_id: i32,
    ) -> Result<Vec<NotificationPreference>, RepoError> {
        use schema::notification_preferences::dsl;
        let mut conn = self.get_conn()?;
        dsl::notification_preferences
            .filter(dsl::user_id.eq(user_id))
            .load::<NotificationPreference>(conn.as_mut())
            .map_err(RepoError::from)
    }

    fn upsert_notification_preference(
        &mut self,
        pref: &NewNotificationPreference,
    ) -> Result<(), RepoError> {
        use schema::notification_preferences::dsl;
        let mut conn = self.get_conn()?;
        diesel::insert_into(dsl::notification_preferences)
            .values(pref)
            .on_conflict((dsl::user_id, dsl::project_id))
            .do_update()
            .set((
                dsl::notify_in_app.eq(pref.notify_in_app),
                dsl::notify_email.eq(pref.notify_email),
            ))
            .execute(conn.as_mut())
            .map_err(RepoError::from)?;
        Ok(())
    }

    fn delete_notification_preference(
        &mut self,
        user_id: i32,
        project_id: i32,
    ) -> Result<(), RepoError> {
        use schema::notification_preferences::dsl;
        let mut conn = self.get_conn()?;
        diesel::delete(
            dsl::notification_preferences
                .filter(dsl::user_id.eq(user_id))
                .filter(dsl::project_id.eq(project_id)),
        )
        .execute(conn.as_mut())
        .map_err(RepoError::from)?;
        Ok(())
    }

    fn get_project_subscribers(
        &self,
        project_id: i32,
    ) -> Result<Vec<NotificationPreference>, RepoError> {
        use schema::notification_preferences::dsl;
        let mut conn = self.get_conn()?;
        dsl::notification_preferences
            .filter(dsl::project_id.eq(project_id))
            .filter(dsl::notify_in_app.eq(true))
            .load::<NotificationPreference>(conn.as_mut())
            .map_err(RepoError::from)
    }
}
