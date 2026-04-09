// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Service for creating and managing user notifications.

use crate::app::{AppState, DieselCachedRepo};
use crate::models::{
    NewNotification, NewNotificationPreference, Notification, NotificationPreference, Requirement,
    User,
};
use crate::repository::errors::RepoError;
use crate::repository::{NotificationRepository, ProjectReviewersRepository, UserRepository};
use crate::services::email_sender;

pub struct NotificationService<'a> {
    state: &'a AppState<DieselCachedRepo>,
}

impl<'a> NotificationService<'a> {
    pub fn new(state: &'a AppState<DieselCachedRepo>) -> Self {
        Self { state }
    }

    // ── Targeted notifications ───────────────────────────────────────────

    /// Notify a user that a requirement has been assigned to them for review.
    pub fn notify_review_assigned(
        &self,
        actor: &User,
        requirement: &Requirement,
        new_reviewer_id: i32,
    ) {
        if new_reviewer_id == actor.id {
            return;
        }
        let title = format!(
            "{} assigned you as reviewer for {}",
            actor.name, requirement.reference_code
        );
        self.create_notification(NewNotification {
            user_id: new_reviewer_id,
            project_id: Some(requirement.project_id),
            notification_type: "review_assigned".into(),
            title,
            body: Some(requirement.title.clone()),
            entity_type: Some("requirement".into()),
            entity_id: Some(requirement.id),
            actor_id: Some(actor.id),
        });
    }

    /// Notify project reviewers that a version has been submitted for approval.
    pub fn notify_approval_requested(
        &self,
        actor: &User,
        requirement: &Requirement,
        project_id: i32,
    ) {
        let reviewer_ids = self
            .state
            .repo_read()
            .list_project_reviewer_ids(project_id)
            .unwrap_or_default();

        let title = format!(
            "{} submitted {} for approval",
            actor.name, requirement.reference_code
        );
        for rid in reviewer_ids {
            if rid == actor.id {
                continue;
            }
            self.create_notification(NewNotification {
                user_id: rid,
                project_id: Some(project_id),
                notification_type: "approval_requested".into(),
                title: title.clone(),
                body: Some(requirement.title.clone()),
                entity_type: Some("requirement".into()),
                entity_id: Some(requirement.id),
                actor_id: Some(actor.id),
            });
        }
    }

    /// Notify the requirement author and reviewer when a comment is added.
    pub fn notify_comment_added(
        &self,
        actor: &User,
        requirement: &Requirement,
        comment_body: &str,
    ) {
        let truncated = if comment_body.len() > 100 {
            format!("{}…", &comment_body[..100])
        } else {
            comment_body.to_string()
        };
        let title = format!("{} commented on {}", actor.name, requirement.reference_code);
        let mut notified = std::collections::HashSet::new();
        notified.insert(actor.id);
        for uid in [requirement.author_id, requirement.reviewer_id] {
            if notified.insert(uid) {
                self.create_notification(NewNotification {
                    user_id: uid,
                    project_id: Some(requirement.project_id),
                    notification_type: "comment_added".into(),
                    title: title.clone(),
                    body: Some(truncated.clone()),
                    entity_type: Some("requirement".into()),
                    entity_id: Some(requirement.id),
                    actor_id: Some(actor.id),
                });
            }
        }
    }

    // ── Project-subscribed notifications ─────────────────────────────────

    /// Notify project subscribers about a requirement lifecycle event.
    pub fn notify_project_event(
        &self,
        actor: &User,
        project_id: i32,
        event_type: &str,
        requirement: &Requirement,
    ) {
        let subscribers = self
            .state
            .repo_read()
            .get_project_subscribers(project_id)
            .unwrap_or_default();

        let action = match event_type {
            "requirement_created" => "created",
            "requirement_updated" => "updated",
            "requirement_deleted" => "deleted",
            _ => event_type,
        };
        let title = format!("{} {} {}", actor.name, action, requirement.reference_code);
        for sub in &subscribers {
            if sub.user_id == actor.id {
                continue;
            }
            self.create_notification(NewNotification {
                user_id: sub.user_id,
                project_id: Some(project_id),
                notification_type: event_type.into(),
                title: title.clone(),
                body: Some(requirement.title.clone()),
                entity_type: Some("requirement".into()),
                entity_id: Some(requirement.id),
                actor_id: Some(actor.id),
            });
        }
    }

    // ── CRUD for API layer ───────────────────────────────────────────────

    pub fn list_for_user(
        &self,
        user_id: i32,
        limit: i64,
        unread_only: bool,
    ) -> Result<Vec<Notification>, RepoError> {
        self.state
            .repo_read()
            .get_notifications_for_user(user_id, limit, unread_only)
    }

    pub fn unread_count(&self, user_id: i32) -> Result<i64, RepoError> {
        self.state.repo_read().count_unread_notifications(user_id)
    }

    pub fn mark_read(&self, id: i32, user_id: i32) -> Result<bool, RepoError> {
        self.state.repo_write().mark_notification_read(id, user_id)
    }

    pub fn mark_all_read(&self, user_id: i32) -> Result<usize, RepoError> {
        self.state.repo_write().mark_all_read(user_id)
    }

    pub fn get_preferences(&self, user_id: i32) -> Result<Vec<NotificationPreference>, RepoError> {
        self.state.repo_read().get_notification_preferences(user_id)
    }

    pub fn set_preference(&self, pref: &NewNotificationPreference) -> Result<(), RepoError> {
        self.state.repo_write().upsert_notification_preference(pref)
    }

    pub fn delete_preference(&self, user_id: i32, project_id: i32) -> Result<(), RepoError> {
        self.state
            .repo_write()
            .delete_notification_preference(user_id, project_id)
    }

    // ── Internal helpers ─────────────────────────────────────────────────

    fn create_notification(&self, new: NewNotification) {
        let user_id = new.user_id;
        let wants_email = self.user_wants_email(user_id, new.project_id);

        let title_for_email = new.title.clone();
        let body_for_email = new.body.clone();

        if let Err(_e) = self.state.repo_write().insert_notification(&new) {
            #[cfg(debug_assertions)]
            eprintln!("notification: failed to insert for user {user_id}: {_e}");
            return;
        }

        if wants_email && email_sender::is_email_enabled() {
            self.try_send_email(user_id, &title_for_email, body_for_email.as_deref());
        }
    }

    fn user_wants_email(&self, user_id: i32, project_id: Option<i32>) -> bool {
        let Some(pid) = project_id else {
            return false;
        };
        self.state
            .repo_read()
            .get_notification_preferences(user_id)
            .unwrap_or_default()
            .iter()
            .any(|p| p.project_id == pid && p.notify_email)
    }

    fn try_send_email(&self, user_id: i32, subject: &str, body: Option<&str>) {
        let email = match self.state.repo_read().get_user_by_id(user_id) {
            Ok(u) => u.email,
            Err(_) => return,
        };
        let body_text = body.unwrap_or(subject);
        if let Err(_e) = email_sender::send_email(&email, subject, body_text) {
            #[cfg(debug_assertions)]
            eprintln!("notification: email to {email} failed: {_e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::diesel_repo_mock::DieselRepoMock;
    use std::sync::{Arc, RwLock};

    type DieselCachedRepo = crate::app::DieselCachedRepo;

    fn state_with_repo(repo: DieselRepoMock) -> AppState<DieselCachedRepo> {
        AppState {
            repo: Arc::new(RwLock::new(DieselCachedRepo::new(repo, 0))),
        }
    }

    fn actor() -> User {
        DieselRepoMock::make_user(1, "alice", "")
    }

    fn requirement(id: i32, project_id: i32) -> Requirement {
        use chrono::NaiveDate;
        let ts = NaiveDate::from_ymd_opt(2024, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        Requirement {
            id,
            current_version_id: None,
            same_as_current: None,
            title: format!("Req {id}"),
            description: String::new(),
            status_id: 1,
            author_id: 1,
            reviewer_id: 2,
            reference_code: format!("REQ-{id:03}"),
            category_id: 1,
            parent_id: None,
            creation_date: ts,
            update_date: ts,
            deadline_date: None,
            applicability_id: 1,
            justification: None,
            project_id,
            approval_state: "draft".into(),
            approved_by: None,
            approved_at: None,
            custom_fields: None,
        }
    }

    #[test]
    fn notify_review_assigned_creates_notification() {
        let mut repo = DieselRepoMock::default();
        repo.users.insert(1, actor());
        repo.users
            .insert(2, DieselRepoMock::make_user(2, "bob", ""));
        let state = state_with_repo(repo);
        let service = NotificationService::new(&state);

        let req = requirement(1, 10);
        service.notify_review_assigned(&actor(), &req, 2);

        let count = service.unread_count(2).unwrap();
        assert_eq!(count, 1);

        let notifs = service.list_for_user(2, 10, false).unwrap();
        assert_eq!(notifs.len(), 1);
        assert_eq!(notifs[0].notification_type, "review_assigned");
        assert!(notifs[0].title.contains("name"));
        assert_eq!(notifs[0].entity_id, Some(1));
    }

    #[test]
    fn notify_review_assigned_skips_self() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = NotificationService::new(&state);

        let req = requirement(1, 10);
        service.notify_review_assigned(&actor(), &req, 1);

        let count = service.unread_count(1).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn notify_comment_added_notifies_author_and_reviewer() {
        let mut repo = DieselRepoMock::default();
        repo.users.insert(1, actor());
        repo.users
            .insert(2, DieselRepoMock::make_user(2, "bob", ""));
        repo.users
            .insert(3, DieselRepoMock::make_user(3, "carol", ""));
        let state = state_with_repo(repo);
        let service = NotificationService::new(&state);

        let mut req = requirement(1, 10);
        req.author_id = 2;
        req.reviewer_id = 3;
        service.notify_comment_added(&actor(), &req, "Great work!");

        assert_eq!(service.unread_count(2).unwrap(), 1);
        assert_eq!(service.unread_count(3).unwrap(), 1);
        assert_eq!(service.unread_count(1).unwrap(), 0);
    }

    #[test]
    fn notify_project_event_only_notifies_subscribers() {
        let mut repo = DieselRepoMock::default();
        repo.users.insert(1, actor());
        repo.users
            .insert(2, DieselRepoMock::make_user(2, "bob", ""));
        repo.users
            .insert(3, DieselRepoMock::make_user(3, "carol", ""));
        repo.notification_preferences.push(NotificationPreference {
            id: 1,
            user_id: 2,
            project_id: 10,
            notify_in_app: true,
            notify_email: false,
        });
        let state = state_with_repo(repo);
        let service = NotificationService::new(&state);

        let req = requirement(1, 10);
        service.notify_project_event(&actor(), 10, "requirement_created", &req);

        assert_eq!(service.unread_count(2).unwrap(), 1);
        assert_eq!(service.unread_count(3).unwrap(), 0);
    }

    #[test]
    fn mark_read_and_mark_all_read() {
        let mut repo = DieselRepoMock::default();
        repo.users.insert(1, actor());
        repo.users
            .insert(2, DieselRepoMock::make_user(2, "bob", ""));
        let state = state_with_repo(repo);
        let service = NotificationService::new(&state);

        let req = requirement(1, 10);
        service.notify_review_assigned(&actor(), &req, 2);
        let req2 = requirement(2, 10);
        service.notify_review_assigned(&actor(), &req2, 2);

        assert_eq!(service.unread_count(2).unwrap(), 2);

        let notifs = service.list_for_user(2, 10, false).unwrap();
        service.mark_read(notifs[0].id, 2).unwrap();
        assert_eq!(service.unread_count(2).unwrap(), 1);

        service.mark_all_read(2).unwrap();
        assert_eq!(service.unread_count(2).unwrap(), 0);
    }

    #[test]
    fn preference_crud() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = NotificationService::new(&state);

        service
            .set_preference(&NewNotificationPreference {
                user_id: 1,
                project_id: 10,
                notify_in_app: true,
                notify_email: false,
            })
            .unwrap();

        let prefs = service.get_preferences(1).unwrap();
        assert_eq!(prefs.len(), 1);
        assert!(prefs[0].notify_in_app);
        assert!(!prefs[0].notify_email);

        service
            .set_preference(&NewNotificationPreference {
                user_id: 1,
                project_id: 10,
                notify_in_app: true,
                notify_email: true,
            })
            .unwrap();

        let prefs = service.get_preferences(1).unwrap();
        assert_eq!(prefs.len(), 1);
        assert!(prefs[0].notify_email);

        service.delete_preference(1, 10).unwrap();
        assert_eq!(service.get_preferences(1).unwrap().len(), 0);
    }
}
