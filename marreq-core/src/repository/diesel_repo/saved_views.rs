// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use super::DieselRepo;
use crate::repository::errors::RepoError;
use crate::repository::SavedViewRepository;
use crate::schema;
use diesel::expression_methods::BoolExpressionMethods;
use diesel::prelude::*;

impl SavedViewRepository for DieselRepo {
    fn list_saved_views_for_user(
        &self,
        project_id: i32,
        user_id: i32,
    ) -> Result<Vec<crate::models::SavedView>, RepoError> {
        use schema::saved_views::dsl;
        let mut conn = self.get_conn()?;
        dsl::saved_views
            .filter(dsl::project_id.eq(project_id))
            .filter(
                dsl::visibility
                    .eq("shared")
                    .or(dsl::owner_id.eq(user_id).and(dsl::visibility.eq("private"))),
            )
            .order((dsl::visibility.asc(), dsl::name.asc(), dsl::id.asc()))
            .load(conn.as_mut())
            .map_err(RepoError::from)
    }

    fn get_saved_view_by_id(&self, id: i32) -> Result<crate::models::SavedView, RepoError> {
        use schema::saved_views::dsl;
        let mut conn = self.get_conn()?;
        dsl::saved_views
            .filter(dsl::id.eq(id))
            .get_result(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })
    }

    fn create_saved_view(
        &mut self,
        project_id: i32,
        owner_id: i32,
        payload: &crate::models::SavedViewPayload,
    ) -> Result<crate::models::SavedView, RepoError> {
        use schema::saved_views::dsl;
        let visibility = crate::saved_view_definition::validate_visibility(&payload.visibility)?;
        let definition =
            crate::saved_view_definition::validate_saved_view_definition(&payload.definition)?;
        let name = payload.name.trim().to_string();
        if name.is_empty() {
            return Err(RepoError::BadInput("name is required".into()));
        }
        let now = chrono::Utc::now().naive_utc();
        let row = crate::models::NewSavedViewRow {
            project_id,
            owner_id,
            name,
            description: payload
                .description
                .as_ref()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
            visibility,
            definition,
            locked: false,
            locked_at: None,
            created_at: now,
            updated_at: now,
        };
        let mut conn = self.get_conn()?;
        diesel::insert_into(dsl::saved_views)
            .values(&row)
            .get_result(conn.as_mut())
            .map_err(RepoError::from)
    }

    fn update_saved_view(
        &mut self,
        id: i32,
        payload: &crate::models::SavedViewPayload,
    ) -> Result<crate::models::SavedView, RepoError> {
        use schema::saved_views::dsl;
        let existing = self.get_saved_view_by_id(id)?;
        if existing.locked {
            return Err(RepoError::BadInput(
                "Saved views used in a baseline are immutable".into(),
            ));
        }
        let visibility = crate::saved_view_definition::validate_visibility(&payload.visibility)?;
        let definition =
            crate::saved_view_definition::validate_saved_view_definition(&payload.definition)?;
        let name = payload.name.trim().to_string();
        if name.is_empty() {
            return Err(RepoError::BadInput("name is required".into()));
        }
        let now = chrono::Utc::now().naive_utc();
        let mut conn = self.get_conn()?;
        let affected = diesel::update(dsl::saved_views.filter(dsl::id.eq(id)))
            .set((
                dsl::name.eq(name),
                dsl::description.eq(payload
                    .description
                    .as_ref()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())),
                dsl::visibility.eq(visibility),
                dsl::definition.eq(definition),
                dsl::updated_at.eq(now),
            ))
            .execute(conn.as_mut())?;
        if affected == 0 {
            return Err(RepoError::NotFound);
        }
        self.get_saved_view_by_id(id)
    }

    fn delete_saved_view(&mut self, id: i32) -> Result<(), RepoError> {
        use schema::saved_views::dsl;
        let existing = self.get_saved_view_by_id(id)?;
        if existing.locked {
            return Err(RepoError::BadInput(
                "Saved views used in a baseline are immutable".into(),
            ));
        }
        let mut conn = self.get_conn()?;
        let affected =
            diesel::delete(dsl::saved_views.filter(dsl::id.eq(id))).execute(conn.as_mut())?;
        if affected == 0 {
            return Err(RepoError::NotFound);
        }
        Ok(())
    }

    fn lock_saved_view(&mut self, id: i32) -> Result<(), RepoError> {
        use schema::saved_views::dsl;
        let now = chrono::Utc::now().naive_utc();
        let mut conn = self.get_conn()?;
        let affected = diesel::update(dsl::saved_views.filter(dsl::id.eq(id)))
            .set((
                dsl::locked.eq(true),
                dsl::locked_at.eq(Some(now)),
                dsl::updated_at.eq(now),
            ))
            .execute(conn.as_mut())?;
        if affected == 0 {
            return Err(RepoError::NotFound);
        }
        Ok(())
    }
}
