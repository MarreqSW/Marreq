// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use crate::auth::ExternalIdentity;
use crate::models::{NewUser, NewWorkspace, User};
use crate::repository::errors::RepoError;
use crate::repository::Repository;
use crate::validation::validate_user;

pub struct UserProvisioningService;

impl UserProvisioningService {
    pub fn create_user<R: Repository>(repo: &mut R, new: &NewUser) -> Result<User, RepoError> {
        Self::create_user_with_personal_workspace(
            repo,
            new,
            crate::deployment::current().assigns_personal_workspace(),
        )
    }

    pub fn create_user_with_personal_workspace<R: Repository>(
        repo: &mut R,
        new: &NewUser,
        assign_personal_workspace: bool,
    ) -> Result<User, RepoError> {
        let id = repo.insert_user(new)?;
        if assign_personal_workspace {
            let mut suffix = 1u32;
            loop {
                let slug = if suffix == 1 {
                    new.username.clone()
                } else {
                    format!("{}-{suffix}", new.username)
                };
                let workspace = NewWorkspace {
                    slug,
                    name: new.name.clone(),
                    owner_user_id: id,
                    kind: "personal".into(),
                };
                match repo.insert_workspace(&workspace) {
                    Ok(_) => break,
                    Err(RepoError::Duplicate(_)) if suffix < 100 => suffix += 1,
                    Err(error) => {
                        // Avoid leaving a user without its mandatory personal
                        // workspace when the second provisioning step fails.
                        let _ = repo.delete_user(id);
                        return Err(error);
                    }
                }
            }
        }
        repo.get_user_by_id(id)
    }

    pub fn provision_external<R: Repository>(
        repo: &mut R,
        identity: &ExternalIdentity,
    ) -> Result<User, RepoError> {
        Self::provision_external_with_personal_workspace(
            repo,
            identity,
            crate::deployment::current().assigns_personal_workspace(),
        )
    }

    pub fn provision_external_with_personal_workspace<R: Repository>(
        repo: &mut R,
        identity: &ExternalIdentity,
        assign_personal_workspace: bool,
    ) -> Result<User, RepoError> {
        let email = identity
            .email
            .as_deref()
            .ok_or_else(|| RepoError::BadInput("provider did not supply an email address".into()))?
            .trim()
            .to_lowercase();
        if repo.get_user_by_email(&email)?.is_some() {
            return Err(RepoError::Duplicate(
                "an account already uses this email; sign in and connect the provider explicitly"
                    .into(),
            ));
        }
        let stem = identity
            .username
            .as_deref()
            .or_else(|| email.split('@').next())
            .unwrap_or("user");
        let username = available_username(repo, stem)?;
        let name = identity
            .display_name
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(&username)
            .trim()
            .to_owned();
        let new = NewUser {
            id: None,
            username: username.clone(),
            name: name.clone(),
            email,
            password_hash: None,
            is_admin: false,
            email_verified: Some(identity.email_verified),
        };
        validate_user(&new).map_err(|error| RepoError::BadInput(error.to_string()))?;
        Self::create_user_with_personal_workspace(repo, &new, assign_personal_workspace)
    }
}

fn available_username<R: Repository>(repo: &R, candidate: &str) -> Result<String, RepoError> {
    let mut stem: String = candidate
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || *character == '_')
        .collect::<String>()
        .to_lowercase();
    stem.truncate(42);
    if stem.len() < 3 {
        stem = "user".into();
    }
    if repo.get_user_by_username(&stem)?.is_none() {
        return Ok(stem);
    }
    for suffix in 2..=9999 {
        let value = format!("{stem}_{suffix}");
        if repo.get_user_by_username(&value)?.is_none() {
            return Ok(value);
        }
    }
    Err(RepoError::Duplicate("unable to allocate a username".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::diesel_repo_mock::DieselRepoMock;
    use crate::repository::UserRepository;
    #[test]
    fn username_collision_gets_stable_suffix() {
        let mut repo = DieselRepoMock::with_users([DieselRepoMock::make_user(1, "alice", "hash")]);
        assert_eq!(available_username(&repo, "Alice").unwrap(), "alice_2");
        let id = repo
            .insert_user(&NewUser {
                id: None,
                username: "alice_2".into(),
                name: "Alice Two".into(),
                email: "two@example.test".into(),
                password_hash: None,
                is_admin: false,
                email_verified: Some(true),
            })
            .unwrap();
        assert!(id > 0);
        assert_eq!(available_username(&repo, "Alice").unwrap(), "alice_3");
    }

    #[test]
    fn cloud_external_provisioning_creates_a_personal_workspace() {
        use crate::repository::WorkspacesRepository;

        let mut repo = DieselRepoMock::default();
        let identity = ExternalIdentity {
            provider_key: "google".into(),
            issuer: "https://accounts.google.com".into(),
            subject: "123".into(),
            email: Some("Alice@example.test".into()),
            email_verified: true,
            username: Some("alice".into()),
            display_name: Some("Alice Example".into()),
        };

        let user = UserProvisioningService::provision_external_with_personal_workspace(
            &mut repo, &identity, true,
        )
        .unwrap();
        let workspace = repo.get_workspace_by_slug("alice").unwrap().unwrap();

        assert_eq!(user.email, "alice@example.test");
        assert!(user.password_hash.is_none());
        assert_eq!(workspace.owner_user_id, user.id);
    }
}
