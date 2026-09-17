// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use crate::auth::{AuthConfig, ExternalIdentity};
use crate::models::{NewLog, NewUserIdentity, User, UserIdentity};
use crate::repository::errors::RepoError;
use crate::repository::Repository;
use crate::services::UserProvisioningService;

pub struct ResolvedExternalUser {
    pub user: User,
    pub identity: UserIdentity,
}

pub fn resolve_login<R: Repository>(
    repo: &mut R,
    external: &ExternalIdentity,
    auto_register: bool,
) -> Result<ResolvedExternalUser, RepoError> {
    if let Some(identity) = repo.get_identity(&external.issuer, &external.subject)? {
        let user = repo.get_user_by_id(identity.user_id)?;
        repo.touch_identity_login(identity.id, chrono::Utc::now().naive_utc())?;
        return Ok(ResolvedExternalUser { user, identity });
    }
    if let Some(email) = external.email.as_deref() {
        if repo.get_user_by_email(email)?.is_some() {
            return Err(RepoError::Duplicate("account_link_required".into()));
        }
    }
    if !auto_register {
        return Err(RepoError::Unauthorized);
    }
    if crate::deployment::current().requires_email_verification() && !external.email_verified {
        return Err(RepoError::BadInput(
            "the provider did not supply a verified email address".into(),
        ));
    }
    let user = UserProvisioningService::provision_external(repo, external)?;
    let identity = NewUserIdentity {
        user_id: user.id,
        provider_key: external.provider_key.clone(),
        issuer: external.issuer.clone(),
        subject: external.subject.clone(),
    };
    let id = match repo.insert_identity(&identity) {
        Ok(id) => id,
        Err(error) => {
            // Provisioning and identity insertion are separate repository calls.
            // Compensate on failure so a uniqueness race cannot leave an
            // unreachable passwordless account behind.
            let _ = repo.delete_user(user.id);
            return Err(error);
        }
    };
    let identity = repo
        .get_identity(&external.issuer, &external.subject)?
        .ok_or(RepoError::NotFound)?;
    debug_assert_eq!(identity.id, id);
    Ok(ResolvedExternalUser { user, identity })
}

pub fn link_identity<R: Repository>(
    repo: &mut R,
    user_id: i32,
    external: &ExternalIdentity,
) -> Result<i32, RepoError> {
    if let Some(existing) = repo.get_identity(&external.issuer, &external.subject)? {
        return if existing.user_id == user_id {
            Ok(existing.id)
        } else {
            Err(RepoError::Unauthorized)
        };
    }
    let id = repo.insert_identity(&NewUserIdentity {
        user_id,
        provider_key: external.provider_key.clone(),
        issuer: external.issuer.clone(),
        subject: external.subject.clone(),
    })?;
    audit_identity(
        repo,
        user_id,
        id,
        "AUTH_IDENTITY_LINK",
        &external.provider_key,
    );
    Ok(id)
}

pub fn unlink_identity<R: Repository>(
    repo: &mut R,
    auth_config: &AuthConfig,
    user_id: i32,
    identity_id: i32,
) -> Result<(), RepoError> {
    let user = repo.get_user_by_id(user_id)?;
    let identities = repo.get_identities_for_user(user_id)?;
    if !identities.iter().any(|identity| identity.id == identity_id) {
        return Err(RepoError::NotFound);
    }
    let usable_methods =
        auth_config.usable_authentication_methods(user.password_hash.is_some(), &identities);
    if !usable_methods.allows_unlinking(identity_id) {
        return Err(RepoError::BadInput(
            "cannot remove the last authentication method".into(),
        ));
    }
    let provider = identities
        .iter()
        .find(|identity| identity.id == identity_id)
        .map(|identity| identity.provider_key.clone())
        .unwrap_or_default();
    if repo.delete_identity(identity_id, user_id)? {
        audit_identity(
            repo,
            user_id,
            identity_id,
            "AUTH_IDENTITY_UNLINK",
            &provider,
        );
        Ok(())
    } else {
        Err(RepoError::NotFound)
    }
}

fn audit_identity<R: Repository>(
    repo: &mut R,
    user_id: i32,
    identity_id: i32,
    action: &str,
    provider: &str,
) {
    let _ = repo.insert_log(&NewLog {
        user_id,
        action_type: action.into(),
        entity_type: "UserIdentity".into(),
        project_id: None,
        entity_id: Some(identity_id),
        old_values: None,
        new_values: Some(serde_json::json!({ "provider": provider }).to_string()),
        description: Some(format!(
            "External identity {action} for provider {provider}"
        )),
        ip_address: None,
        user_agent: None,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{AuthProviderConfig, ProviderKind};
    use crate::repository::diesel_repo_mock::DieselRepoMock;
    use crate::repository::ExternalIdentityRepository;

    fn external(subject: &str, email: &str) -> ExternalIdentity {
        external_for("github", subject, email)
    }

    fn external_for(provider_key: &str, subject: &str, email: &str) -> ExternalIdentity {
        ExternalIdentity {
            provider_key: provider_key.into(),
            issuer: if provider_key == "github" {
                "https://github.com".into()
            } else {
                format!("https://{provider_key}.example.test")
            },
            subject: subject.into(),
            email: Some(email.into()),
            email_verified: true,
            username: Some("alice".into()),
            display_name: Some("Alice".into()),
        }
    }

    fn auth_config(password_enabled: bool, providers: &[&str]) -> AuthConfig {
        AuthConfig::new(
            password_enabled,
            providers
                .iter()
                .map(|key| AuthProviderConfig {
                    key: (*key).into(),
                    display_name: (*key).into(),
                    kind: ProviderKind::OAuth2 {
                        issuer: format!("https://{key}.example.test"),
                        authorization_url: format!("https://{key}.example.test/authorize"),
                        token_url: format!("https://{key}.example.test/token"),
                        userinfo_url: format!("https://{key}.example.test/user"),
                    },
                    client_id: "client-id".into(),
                    client_secret: "client-secret".into(),
                    auto_register: true,
                })
                .collect(),
        )
    }

    #[test]
    fn identity_is_resolved_by_issuer_and_subject() {
        let user = DieselRepoMock::make_user(1, "alice", "hash");
        let mut repo = DieselRepoMock::with_users([user]);
        let id = link_identity(&mut repo, 1, &external("42", "alice@example.test")).unwrap();
        let found = repo
            .get_identity("https://github.com", "42")
            .unwrap()
            .unwrap();
        assert_eq!(found.id, id);
        assert_eq!(found.user_id, 1);
    }

    #[test]
    fn identity_owned_by_another_user_cannot_be_linked() {
        let mut other = DieselRepoMock::make_user(2, "bob", "hash");
        other.email = "bob@example.test".into();
        let mut repo =
            DieselRepoMock::with_users([DieselRepoMock::make_user(1, "alice", "hash"), other]);
        link_identity(&mut repo, 1, &external("42", "alice@example.test")).unwrap();
        assert!(matches!(
            link_identity(&mut repo, 2, &external("42", "alice@example.test")),
            Err(RepoError::Unauthorized)
        ));
    }

    #[test]
    fn one_user_can_link_multiple_external_identities() {
        let mut repo = DieselRepoMock::with_users([DieselRepoMock::make_user(1, "alice", "hash")]);
        link_identity(&mut repo, 1, &external("42", "alice@example.test")).unwrap();
        link_identity(&mut repo, 1, &external("84", "alice@example.test")).unwrap();

        assert_eq!(repo.get_identities_for_user(1).unwrap().len(), 2);
    }

    #[test]
    fn unknown_identity_email_collision_requires_explicit_link() {
        let mut repo = DieselRepoMock::with_users([DieselRepoMock::make_user(1, "alice", "hash")]);
        assert!(
            matches!(resolve_login(&mut repo, &external("42", "email@example.com"), true), Err(RepoError::Duplicate(message)) if message == "account_link_required")
        );
        assert!(repo
            .get_identity("https://github.com", "42")
            .unwrap()
            .is_none());
    }

    #[test]
    fn server_policy_rejects_unknown_identity_when_auto_registration_is_disabled() {
        let mut repo = DieselRepoMock::default();
        assert!(matches!(
            resolve_login(&mut repo, &external("42", "alice@example.test"), false),
            Err(RepoError::Unauthorized)
        ));
        assert!(repo
            .get_identity("https://github.com", "42")
            .unwrap()
            .is_none());
    }

    #[test]
    fn enabled_password_allows_unlinking_only_external_identity() {
        let mut repo = DieselRepoMock::with_users([DieselRepoMock::make_user(1, "alice", "hash")]);
        let id = link_identity(&mut repo, 1, &external("42", "alice@example.test")).unwrap();

        assert!(unlink_identity(&mut repo, &auth_config(true, &["github"]), 1, id).is_ok());
    }

    #[test]
    fn disabled_password_does_not_allow_unlinking_only_external_identity() {
        let mut repo = DieselRepoMock::with_users([DieselRepoMock::make_user(1, "alice", "hash")]);
        let id = link_identity(&mut repo, 1, &external("42", "alice@example.test")).unwrap();

        assert!(matches!(
            unlink_identity(&mut repo, &auth_config(false, &["github"]), 1, id),
            Err(RepoError::BadInput(_))
        ));
    }

    #[test]
    fn external_only_user_cannot_unlink_only_enabled_identity() {
        let mut user = DieselRepoMock::make_user(1, "alice", "hash");
        user.password_hash = None;
        let mut repo = DieselRepoMock::with_users([user]);
        let id = link_identity(&mut repo, 1, &external("42", "alice@example.test")).unwrap();

        assert!(matches!(
            unlink_identity(&mut repo, &auth_config(true, &["github"]), 1, id),
            Err(RepoError::BadInput(_))
        ));
    }

    #[test]
    fn two_enabled_external_identities_allow_unlinking_one() {
        let mut user = DieselRepoMock::make_user(1, "alice", "hash");
        user.password_hash = None;
        let mut repo = DieselRepoMock::with_users([user]);
        let github = link_identity(&mut repo, 1, &external("42", "alice@example.test")).unwrap();
        link_identity(
            &mut repo,
            1,
            &external_for("gitlab", "84", "alice@example.test"),
        )
        .unwrap();

        assert!(unlink_identity(
            &mut repo,
            &auth_config(true, &["github", "gitlab"]),
            1,
            github,
        )
        .is_ok());
    }

    #[test]
    fn disabled_provider_identity_does_not_make_enabled_identity_removable() {
        let mut user = DieselRepoMock::make_user(1, "alice", "hash");
        user.password_hash = None;
        let mut repo = DieselRepoMock::with_users([user]);
        let github = link_identity(&mut repo, 1, &external("42", "alice@example.test")).unwrap();
        link_identity(
            &mut repo,
            1,
            &external_for("disabled", "84", "alice@example.test"),
        )
        .unwrap();

        assert!(matches!(
            unlink_identity(&mut repo, &auth_config(true, &["github"]), 1, github),
            Err(RepoError::BadInput(_))
        ));
    }

    #[test]
    fn stale_identity_can_be_removed_when_an_enabled_identity_remains() {
        let mut user = DieselRepoMock::make_user(1, "alice", "hash");
        user.password_hash = None;
        let mut repo = DieselRepoMock::with_users([user]);
        link_identity(&mut repo, 1, &external("42", "alice@example.test")).unwrap();
        let stale = link_identity(
            &mut repo,
            1,
            &external_for("disabled", "84", "alice@example.test"),
        )
        .unwrap();

        assert!(unlink_identity(&mut repo, &auth_config(true, &["github"]), 1, stale).is_ok());
    }
}
