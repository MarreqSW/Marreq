use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::{Duration, NaiveDateTime};
use rand::RngCore;
use sha2::{Digest, Sha256};
use url::Url;

use crate::models::*;
use crate::repository::errors::RepoError;
use crate::repository::DelegatedOAuthRepository;

pub const SCOPES: &[&str] = &[
    "projects:read",
    "requirements:read",
    "requirements:write",
    "requirements:approve",
    "verifications:read",
    "verifications:write",
    "traceability:read",
    "traceability:write",
    "baselines:read",
    "baselines:write",
];
pub const AUTHORIZATION_CODE_SECONDS: i64 = 300;
pub const ACCESS_TOKEN_SECONDS: i64 = 900;
pub const REFRESH_TOKEN_DAYS: i64 = 30;

#[derive(Debug, thiserror::Error)]
pub enum DelegatedOAuthError {
    #[error("invalid_request")]
    InvalidRequest,
    #[error("invalid_client")]
    InvalidClient,
    #[error("invalid_grant")]
    InvalidGrant,
    #[error("invalid_scope")]
    InvalidScope,
    #[error("invalid_target")]
    InvalidTarget,
    #[error("access_denied")]
    AccessDenied,
    #[error("temporarily_unavailable")]
    Repository(#[from] RepoError),
}

#[derive(Debug, Clone)]
pub struct AuthorizeRequest {
    pub client_id: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
    pub resource: String,
    pub code_challenge: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: &'static str,
    pub expires_in: i64,
    pub refresh_token: String,
    pub scope: String,
}

#[derive(Debug, Clone)]
pub struct DelegatedPrincipal {
    pub user: User,
    pub client_id: String,
    pub grant_id: i32,
    pub scopes: Vec<String>,
    pub resource: String,
}

fn random_secret() -> String {
    let mut bytes = [0_u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn hash_secret(secret: &str) -> String {
    let digest = Sha256::digest(secret.as_bytes());
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

fn parse_scopes(raw: &str) -> Result<Vec<String>, DelegatedOAuthError> {
    let mut scopes = raw
        .split_whitespace()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    scopes.sort();
    scopes.dedup();
    if scopes.is_empty() || scopes.iter().any(|scope| !SCOPES.contains(&scope.as_str())) {
        return Err(DelegatedOAuthError::InvalidScope);
    }
    Ok(scopes)
}

pub fn validate_redirect_uri(value: &str) -> Result<(), DelegatedOAuthError> {
    let url = Url::parse(value).map_err(|_| DelegatedOAuthError::InvalidRequest)?;
    let local = url.scheme() == "http"
        && matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"));
    if url.fragment().is_some() || (url.scheme() != "https" && !local) {
        return Err(DelegatedOAuthError::InvalidRequest);
    }
    Ok(())
}

fn exact_redirect(client: &OAuthClient, redirect: &str) -> bool {
    client
        .redirect_uris
        .as_array()
        .is_some_and(|values| values.iter().any(|value| value.as_str() == Some(redirect)))
}

pub fn register_client<R: DelegatedOAuthRepository>(
    repo: &mut R,
    name: &str,
    redirects: Vec<String>,
) -> Result<OAuthClient, DelegatedOAuthError> {
    let name = name.trim();
    if name.is_empty()
        || name.len() > 100
        || name.chars().any(char::is_control)
        || redirects.is_empty()
        || redirects.len() > 10
        || redirects.iter().any(|redirect| redirect.len() > 2048)
    {
        return Err(DelegatedOAuthError::InvalidRequest);
    }
    let mut unique_redirects = redirects.clone();
    unique_redirects.sort();
    unique_redirects.dedup();
    if unique_redirects.len() != redirects.len() {
        return Err(DelegatedOAuthError::InvalidRequest);
    }
    for redirect in &redirects {
        validate_redirect_uri(redirect)?;
    }
    let id = format!("mcp_{}", random_secret());
    repo.insert_oauth_client(&NewOAuthClient {
        client_id: id.clone(),
        name: name.to_owned(),
        redirect_uris: serde_json::json!(redirects),
    })?;
    repo.get_oauth_client(&id).map_err(Into::into)
}

pub fn authorize<R: DelegatedOAuthRepository>(
    repo: &mut R,
    user_id: i32,
    request: AuthorizeRequest,
    expected_resource: &str,
    now: NaiveDateTime,
) -> Result<String, DelegatedOAuthError> {
    if request.resource != expected_resource {
        return Err(DelegatedOAuthError::InvalidTarget);
    }
    if request.code_challenge.len() < 43
        || request.code_challenge.len() > 128
        || !request
            .code_challenge
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~'))
    {
        return Err(DelegatedOAuthError::InvalidRequest);
    }
    let client = repo
        .get_oauth_client(&request.client_id)
        .map_err(|_| DelegatedOAuthError::InvalidClient)?;
    if !exact_redirect(&client, &request.redirect_uri) {
        return Err(DelegatedOAuthError::InvalidRequest);
    }
    let scopes = parse_scopes(&request.scopes.join(" "))?;
    let grant = repo.upsert_oauth_grant(&NewOAuthGrant {
        user_id,
        client_id: request.client_id.clone(),
        scopes: scopes.clone(),
        resource: request.resource.clone(),
    })?;
    let raw = random_secret();
    repo.insert_oauth_code(&NewOAuthAuthorizationCode {
        code_hash: hash_secret(&raw),
        grant_id: grant.id,
        client_id: request.client_id,
        redirect_uri: request.redirect_uri,
        code_challenge: request.code_challenge,
        scopes,
        resource: request.resource,
        expires_at: now + Duration::seconds(AUTHORIZATION_CODE_SECONDS),
    })?;
    Ok(raw)
}

fn token_pair(
    code: &OAuthAuthorizationCode,
    family: String,
    now: NaiveDateTime,
) -> (TokenResponse, NewOAuthAccessToken, NewOAuthRefreshToken) {
    let access = random_secret();
    let refresh = random_secret();
    let response = TokenResponse {
        access_token: access.clone(),
        token_type: "Bearer",
        expires_in: ACCESS_TOKEN_SECONDS,
        refresh_token: refresh.clone(),
        scope: code.scopes.join(" "),
    };
    let access_row = NewOAuthAccessToken {
        token_hash: hash_secret(&access),
        grant_id: code.grant_id,
        client_id: code.client_id.clone(),
        scopes: code.scopes.clone(),
        resource: code.resource.clone(),
        expires_at: now + Duration::seconds(ACCESS_TOKEN_SECONDS),
    };
    let refresh_row = NewOAuthRefreshToken {
        token_hash: hash_secret(&refresh),
        family_id: family,
        grant_id: code.grant_id,
        client_id: code.client_id.clone(),
        scopes: code.scopes.clone(),
        resource: code.resource.clone(),
        expires_at: now + Duration::days(REFRESH_TOKEN_DAYS),
    };
    (response, access_row, refresh_row)
}

pub fn exchange_code<R: DelegatedOAuthRepository>(
    repo: &mut R,
    raw_code: &str,
    verifier: &str,
    client_id: &str,
    redirect_uri: &str,
    resource: &str,
    now: NaiveDateTime,
) -> Result<TokenResponse, DelegatedOAuthError> {
    if verifier.len() < 43
        || verifier.len() > 128
        || !verifier
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~'))
    {
        return Err(DelegatedOAuthError::InvalidGrant);
    }
    let hash = hash_secret(raw_code);
    let mut code = repo
        .get_oauth_code(&hash)
        .map_err(|_| DelegatedOAuthError::InvalidGrant)?;
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    if code.used_at.is_some()
        || code.expires_at <= now
        || code.client_id != client_id
        || code.redirect_uri != redirect_uri
        || code.resource != resource
        || code.code_challenge != challenge
    {
        return Err(DelegatedOAuthError::InvalidGrant);
    }
    let grant = repo
        .get_oauth_grant(code.grant_id)
        .map_err(|_| DelegatedOAuthError::InvalidGrant)?;
    if grant.revoked_at.is_some() || grant.resource != resource || grant.client_id != client_id {
        return Err(DelegatedOAuthError::InvalidGrant);
    }
    code.scopes.retain(|scope| grant.scopes.contains(scope));
    if code.scopes.is_empty() {
        return Err(DelegatedOAuthError::InvalidGrant);
    }
    if !repo.consume_oauth_code(&hash, now)? {
        return Err(DelegatedOAuthError::InvalidGrant);
    }
    let (response, access, refresh) = token_pair(&code, random_secret(), now);
    repo.insert_oauth_tokens(&access, &refresh)?;
    Ok(response)
}

pub fn refresh<R: DelegatedOAuthRepository>(
    repo: &mut R,
    raw: &str,
    client_id: &str,
    resource: &str,
    now: NaiveDateTime,
) -> Result<TokenResponse, DelegatedOAuthError> {
    let hash = hash_secret(raw);
    let (old, grant) = repo
        .get_oauth_refresh_token(&hash)
        .map_err(|_| DelegatedOAuthError::InvalidGrant)?;
    if old.used_at.is_some() || old.revoked_at.is_some() {
        repo.revoke_oauth_refresh_family(&old.family_id, now)?;
        return Err(DelegatedOAuthError::InvalidGrant);
    }
    if old.expires_at <= now
        || old.client_id != client_id
        || old.resource != resource
        || grant.revoked_at.is_some()
    {
        return Err(DelegatedOAuthError::InvalidGrant);
    }
    let effective_scopes = old
        .scopes
        .iter()
        .filter(|scope| grant.scopes.contains(scope))
        .cloned()
        .collect::<Vec<_>>();
    if effective_scopes.is_empty() {
        return Err(DelegatedOAuthError::InvalidGrant);
    }
    let code = OAuthAuthorizationCode {
        code_hash: String::new(),
        grant_id: old.grant_id,
        client_id: old.client_id.clone(),
        redirect_uri: String::new(),
        code_challenge: String::new(),
        scopes: effective_scopes,
        resource: old.resource.clone(),
        expires_at: now,
        used_at: None,
        created_at: now,
    };
    let (response, access, next) = token_pair(&code, old.family_id.clone(), now);
    if !repo.rotate_oauth_refresh_token(&hash, &access, &next, now)? {
        repo.revoke_oauth_refresh_family(&old.family_id, now)?;
        return Err(DelegatedOAuthError::InvalidGrant);
    }
    Ok(response)
}

pub fn validate_access<R: DelegatedOAuthRepository>(
    repo: &R,
    raw: &str,
    resource: &str,
    required_scope: Option<&str>,
    now: NaiveDateTime,
) -> Result<DelegatedPrincipal, DelegatedOAuthError> {
    let (token, grant, user) = repo
        .get_oauth_access_token(&hash_secret(raw))
        .map_err(|_| DelegatedOAuthError::AccessDenied)?;
    if token.expires_at <= now
        || token.resource != resource
        || grant.resource != resource
        || grant.revoked_at.is_some()
        || token.client_id != grant.client_id
    {
        return Err(DelegatedOAuthError::AccessDenied);
    }
    let effective_scopes = token
        .scopes
        .into_iter()
        .filter(|scope| grant.scopes.contains(scope))
        .collect::<Vec<_>>();
    if required_scope.is_some_and(|scope| !effective_scopes.iter().any(|held| held == scope)) {
        return Err(DelegatedOAuthError::InvalidScope);
    }
    Ok(DelegatedPrincipal {
        user,
        client_id: token.client_id,
        grant_id: token.grant_id,
        scopes: effective_scopes,
        resource: token.resource,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::diesel_repo_mock::DieselRepoMock;
    #[test]
    fn rejects_non_s256_redirects_and_unknown_scopes() {
        assert!(validate_redirect_uri("http://example.com/callback").is_err());
        assert!(validate_redirect_uri("http://localhost:9999/callback").is_ok());
        assert!(parse_scopes("requirements:read root:admin").is_err());
    }
    #[test]
    fn authorization_codes_are_single_use_and_pkce_bound() {
        let now = chrono::Utc::now().naive_utc();
        let mut repo = DieselRepoMock::default();
        let client = register_client(
            &mut repo,
            "Test",
            vec!["http://localhost:9000/callback".into()],
        )
        .unwrap();
        let verifier = "a".repeat(64);
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        let req = AuthorizeRequest {
            client_id: client.client_id.clone(),
            redirect_uri: "http://localhost:9000/callback".into(),
            scopes: vec!["requirements:read".into()],
            resource: "https://marreq.test/mcp".into(),
            code_challenge: challenge,
        };
        let code = authorize(&mut repo, 1, req, "https://marreq.test/mcp", now).unwrap();
        assert!(exchange_code(
            &mut repo,
            &code,
            "wrong",
            &client.client_id,
            "http://localhost:9000/callback",
            "https://marreq.test/mcp",
            now
        )
        .is_err());
        assert!(exchange_code(
            &mut repo,
            &code,
            &verifier,
            &client.client_id,
            "http://localhost:9000/callback",
            "https://marreq.test/mcp",
            now
        )
        .is_ok());
        assert!(exchange_code(
            &mut repo,
            &code,
            &verifier,
            &client.client_id,
            "http://localhost:9000/callback",
            "https://marreq.test/mcp",
            now
        )
        .is_err());
    }

    #[test]
    fn rejects_invalid_pkce_verifier_shapes() {
        let now = chrono::Utc::now().naive_utc();
        for verifier in [
            String::new(),
            "a".repeat(42),
            "a".repeat(129),
            format!("{}!", "a".repeat(42)),
        ] {
            let mut repo = DieselRepoMock::default();
            let client = register_client(
                &mut repo,
                "Test",
                vec!["http://localhost:9000/callback".into()],
            )
            .unwrap();
            let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
            let code = authorize(
                &mut repo,
                1,
                AuthorizeRequest {
                    client_id: client.client_id.clone(),
                    redirect_uri: "http://localhost:9000/callback".into(),
                    scopes: vec!["requirements:read".into()],
                    resource: "https://marreq.test/mcp".into(),
                    code_challenge: challenge,
                },
                "https://marreq.test/mcp",
                now,
            )
            .unwrap();
            assert!(matches!(
                exchange_code(
                    &mut repo,
                    &code,
                    &verifier,
                    &client.client_id,
                    "http://localhost:9000/callback",
                    "https://marreq.test/mcp",
                    now,
                ),
                Err(DelegatedOAuthError::InvalidGrant)
            ));
        }
    }

    #[test]
    fn scope_downgrade_invalidates_issued_credentials() {
        let now = chrono::Utc::now().naive_utc();
        let resource = "https://marreq.test/mcp";
        let redirect = "http://localhost:9000/callback";
        let verifier = "v".repeat(64);
        let mut repo = DieselRepoMock::default();
        repo.users
            .insert(1, DieselRepoMock::make_user(1, "user", "hash"));
        let client = register_client(&mut repo, "Test", vec![redirect.into()]).unwrap();
        let authorize_with = |repo: &mut DieselRepoMock, scopes: Vec<String>| {
            authorize(
                repo,
                1,
                AuthorizeRequest {
                    client_id: client.client_id.clone(),
                    redirect_uri: redirect.into(),
                    scopes,
                    resource: resource.into(),
                    code_challenge: URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes())),
                },
                resource,
                now,
            )
            .unwrap()
        };
        let code = authorize_with(
            &mut repo,
            vec!["requirements:read".into(), "requirements:write".into()],
        );
        let tokens = exchange_code(
            &mut repo,
            &code,
            &verifier,
            &client.client_id,
            redirect,
            resource,
            now,
        )
        .unwrap();
        assert!(validate_access(
            &repo,
            &tokens.access_token,
            resource,
            Some("requirements:write"),
            now,
        )
        .is_ok());

        authorize_with(&mut repo, vec!["requirements:read".into()]);

        assert!(validate_access(
            &repo,
            &tokens.access_token,
            resource,
            Some("requirements:write"),
            now,
        )
        .is_err());
        assert!(refresh(
            &mut repo,
            &tokens.refresh_token,
            &client.client_id,
            resource,
            now,
        )
        .is_err());
    }
}
