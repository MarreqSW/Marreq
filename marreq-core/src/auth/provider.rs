// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use super::{AuthProviderConfig, ExternalIdentity, OAuthTransaction, ProviderKind};
use openidconnect::core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata};
use openidconnect::{
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope, TokenResponse,
};
use serde::Deserialize;

pub async fn authorization_url(
    provider: &AuthProviderConfig,
    transaction: &OAuthTransaction,
    public_base_url: &str,
) -> Result<String, String> {
    let redirect = redirect_uri(public_base_url, &provider.key)?;
    match &provider.kind {
        ProviderKind::Oidc { issuer } => {
            let client = oidc_client(provider, issuer, &redirect).await?;
            let challenge = PkceCodeChallenge::from_code_verifier_sha256(&PkceCodeVerifier::new(
                transaction.pkce_verifier.clone(),
            ));
            let state = transaction.state.clone();
            let nonce = transaction.nonce.clone();
            let (url, _, _) = client
                .authorize_url(
                    CoreAuthenticationFlow::AuthorizationCode,
                    || CsrfToken::new(state),
                    || Nonce::new(nonce),
                )
                .add_scope(Scope::new("email".into()))
                .add_scope(Scope::new("profile".into()))
                .set_pkce_challenge(challenge)
                .url();
            Ok(url.to_string())
        }
        ProviderKind::OAuth2 {
            authorization_url,
            token_url,
            ..
        } => {
            let client = oauth_client(provider, authorization_url, token_url, &redirect)?;
            let scope = if provider.key == "github" {
                "read:user user:email"
            } else {
                "read_user"
            };
            let challenge = oauth2::PkceCodeChallenge::from_code_verifier_sha256(
                &oauth2::PkceCodeVerifier::new(transaction.pkce_verifier.clone()),
            );
            let (url, _) = client
                .authorize_url(|| oauth2::CsrfToken::new(transaction.state.clone()))
                .add_scope(oauth2::Scope::new(scope.into()))
                .set_pkce_challenge(challenge)
                .url();
            Ok(url.to_string())
        }
    }
}

pub async fn exchange_code(
    provider: &AuthProviderConfig,
    code: &str,
    transaction: &OAuthTransaction,
    public_base_url: &str,
) -> Result<ExternalIdentity, String> {
    let redirect = redirect_uri(public_base_url, &provider.key)?;
    match &provider.kind {
        ProviderKind::Oidc { issuer } => {
            let client = oidc_client(provider, issuer, &redirect).await?;
            let http = oidc_http_client()?;
            let token = client
                .exchange_code(AuthorizationCode::new(code.to_owned()))
                .map_err(|_| "provider has no token endpoint".to_string())?
                .set_pkce_verifier(PkceCodeVerifier::new(transaction.pkce_verifier.clone()))
                .request_async(&http)
                .await
                .map_err(|_| "provider token exchange failed".to_string())?;
            let id_token = token
                .id_token()
                .ok_or_else(|| "provider did not return an ID token".to_string())?;
            let claims = id_token
                .claims(
                    &client.id_token_verifier(),
                    &Nonce::new(transaction.nonce.clone()),
                )
                .map_err(|_| "ID token validation failed".to_string())?;
            Ok(ExternalIdentity {
                provider_key: provider.key.clone(),
                issuer: issuer.clone(),
                subject: claims.subject().as_str().to_owned(),
                email: claims.email().map(|email| email.as_str().to_owned()),
                email_verified: claims.email_verified().unwrap_or(false),
                username: claims
                    .preferred_username()
                    .map(|name| name.as_str().to_owned()),
                display_name: None,
            })
        }
        ProviderKind::OAuth2 {
            issuer,
            token_url,
            userinfo_url,
            ..
        } => {
            oauth_profile(
                provider,
                issuer,
                token_url,
                userinfo_url,
                code,
                &redirect,
                transaction,
            )
            .await
        }
    }
}

async fn oidc_client(
    provider: &AuthProviderConfig,
    issuer: &str,
    redirect: &str,
) -> Result<
    CoreClient<
        openidconnect::EndpointSet,
        openidconnect::EndpointNotSet,
        openidconnect::EndpointNotSet,
        openidconnect::EndpointNotSet,
        openidconnect::EndpointMaybeSet,
        openidconnect::EndpointMaybeSet,
    >,
    String,
> {
    let issuer_url =
        IssuerUrl::new(issuer.to_owned()).map_err(|_| "invalid OIDC issuer".to_string())?;
    let http = oidc_http_client()?;
    let metadata = CoreProviderMetadata::discover_async(issuer_url.clone(), &http)
        .await
        .map_err(|_| "OIDC discovery failed".to_string())?;
    if !issuer_matches(issuer_url.as_str(), metadata.issuer().as_str()) {
        return Err("OIDC discovery issuer mismatch".into());
    }
    Ok(CoreClient::from_provider_metadata(
        metadata,
        ClientId::new(provider.client_id.clone()),
        Some(ClientSecret::new(provider.client_secret().to_owned())),
    )
    .set_redirect_uri(
        RedirectUrl::new(redirect.to_owned()).map_err(|_| "invalid callback URL".to_string())?,
    ))
}

fn issuer_matches(configured: &str, discovered: &str) -> bool {
    configured == discovered
}

fn oidc_http_client() -> Result<openidconnect::reqwest::Client, String> {
    openidconnect::reqwest::ClientBuilder::new()
        .redirect(openidconnect::reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "failed to build OIDC HTTP client".to_string())
}

fn redirect_uri(base: &str, provider: &str) -> Result<String, String> {
    let base = base.trim_end_matches('/');
    let url = format!("{base}/api/auth/external/{provider}/callback");
    url::Url::parse(&url).map_err(|_| "invalid public base URL".to_string())?;
    Ok(url)
}

#[derive(Deserialize)]
struct OAuthProfile {
    id: serde_json::Value,
    login: Option<String>,
    username: Option<String>,
    name: Option<String>,
    email: Option<String>,
    confirmed_at: Option<String>,
}
#[derive(Deserialize)]
struct GitHubEmail {
    email: String,
    primary: bool,
    verified: bool,
}

async fn oauth_profile(
    provider: &AuthProviderConfig,
    issuer: &str,
    token_url: &str,
    userinfo_url: &str,
    code: &str,
    redirect: &str,
    transaction: &OAuthTransaction,
) -> Result<ExternalIdentity, String> {
    let oauth_http = oauth2::reqwest::ClientBuilder::new()
        .redirect(oauth2::reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "failed to build OAuth HTTP client".to_string())?;
    let profile_http = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "failed to build provider profile HTTP client".to_string())?;
    let authorization_url = match &provider.kind {
        ProviderKind::OAuth2 {
            authorization_url, ..
        } => authorization_url,
        ProviderKind::Oidc { .. } => return Err("provider is not OAuth 2.0".into()),
    };
    let client = oauth_client(provider, authorization_url, token_url, redirect)?;
    let token = client
        .exchange_code(oauth2::AuthorizationCode::new(code.to_owned()))
        .set_pkce_verifier(oauth2::PkceCodeVerifier::new(
            transaction.pkce_verifier.clone(),
        ))
        .request_async(&oauth_http)
        .await
        .map_err(|_| "provider token exchange failed".to_string())?;
    let access_token = oauth2::TokenResponse::access_token(&token).secret();
    let profile = profile_http
        .get(userinfo_url)
        .bearer_auth(access_token)
        .header("Accept", "application/json")
        .header("User-Agent", "Marreq")
        .send()
        .await
        .map_err(|_| "provider profile request failed".to_string())?
        .error_for_status()
        .map_err(|_| "provider profile request rejected".to_string())?
        .json::<OAuthProfile>()
        .await
        .map_err(|_| "invalid provider profile response".to_string())?;
    let subject = match profile.id {
        serde_json::Value::Number(value) => value.to_string(),
        serde_json::Value::String(value) if !value.is_empty() => value,
        _ => return Err("provider profile has no immutable id".into()),
    };
    let (email, email_verified) = if provider.key == "github" {
        let emails = profile_http
            .get("https://api.github.com/user/emails")
            .bearer_auth(access_token)
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "Marreq")
            .send()
            .await
            .ok()
            .and_then(|response| response.error_for_status().ok());
        if let Some(response) = emails {
            let entries = response
                .json::<Vec<GitHubEmail>>()
                .await
                .unwrap_or_default();
            entries
                .into_iter()
                .find(|entry| entry.primary && entry.verified)
                .map(|entry| (Some(entry.email), true))
                .unwrap_or((profile.email, false))
        } else {
            (profile.email, false)
        }
    } else {
        (profile.email, profile.confirmed_at.is_some())
    };
    Ok(ExternalIdentity {
        provider_key: provider.key.clone(),
        issuer: issuer.into(),
        subject,
        email,
        email_verified,
        username: profile.login.or(profile.username),
        display_name: profile.name,
    })
}

fn oauth_client(
    provider: &AuthProviderConfig,
    authorization_url: &str,
    token_url: &str,
    redirect: &str,
) -> Result<
    oauth2::basic::BasicClient<
        oauth2::EndpointSet,
        oauth2::EndpointNotSet,
        oauth2::EndpointNotSet,
        oauth2::EndpointNotSet,
        oauth2::EndpointSet,
    >,
    String,
> {
    Ok(
        oauth2::basic::BasicClient::new(oauth2::ClientId::new(provider.client_id.clone()))
            .set_client_secret(oauth2::ClientSecret::new(
                provider.client_secret().to_owned(),
            ))
            .set_auth_uri(
                oauth2::AuthUrl::new(authorization_url.to_owned())
                    .map_err(|_| "invalid provider authorization URL".to_string())?,
            )
            .set_token_uri(
                oauth2::TokenUrl::new(token_url.to_owned())
                    .map_err(|_| "invalid provider token URL".to_string())?,
            )
            .set_redirect_uri(
                oauth2::RedirectUrl::new(redirect.to_owned())
                    .map_err(|_| "invalid callback URL".to_string())?,
            ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::OAuthOperation;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};

    fn provider(base: &str) -> AuthProviderConfig {
        AuthProviderConfig {
            key: "gitlab".into(),
            display_name: "Test GitLab".into(),
            kind: ProviderKind::OAuth2 {
                issuer: "https://gitlab.example.test".into(),
                authorization_url: format!("{base}/authorize"),
                token_url: format!("{base}/token"),
                userinfo_url: format!("{base}/user"),
            },
            client_id: "client-id".into(),
            client_secret: "client-secret".into(),
            auto_register: true,
        }
    }

    fn oidc_provider(issuer: &str) -> AuthProviderConfig {
        AuthProviderConfig {
            key: "oidc".into(),
            display_name: "Company SSO".into(),
            kind: ProviderKind::Oidc {
                issuer: issuer.into(),
            },
            client_id: "client-id".into(),
            client_secret: "client-secret".into(),
            auto_register: false,
        }
    }

    fn transaction() -> OAuthTransaction {
        OAuthTransaction::new(
            "gitlab",
            "/projects/example",
            OAuthOperation::Login,
            chrono::Utc::now().naive_utc(),
        )
        .unwrap()
    }

    #[rocket::async_test]
    async fn oauth_authorization_uses_state_s256_pkce_and_fixed_callback() {
        let provider = provider("http://127.0.0.1:9");
        let transaction = transaction();
        let authorization =
            authorization_url(&provider, &transaction, "https://marreq.example.test")
                .await
                .unwrap();
        let url = url::Url::parse(&authorization).unwrap();
        let query = url
            .query_pairs()
            .collect::<std::collections::HashMap<_, _>>();

        assert_eq!(query.get("response_type").unwrap(), "code");
        assert_eq!(query.get("state").unwrap(), &transaction.state);
        assert_eq!(query.get("code_challenge_method").unwrap(), "S256");
        assert_eq!(
            query.get("code_challenge").unwrap(),
            &crate::auth::pkce_challenge(&transaction.pkce_verifier)
        );
        assert_eq!(
            query.get("redirect_uri").unwrap(),
            "https://marreq.example.test/api/auth/external/gitlab/callback"
        );
    }

    #[rocket::async_test]
    async fn oauth_exchange_sends_pkce_and_uses_immutable_profile_id() {
        let transaction = transaction();
        let (base, server) = mock_oauth_server(transaction.pkce_verifier.clone(), true);

        let identity = exchange_code(
            &provider(&base),
            "authorization-code",
            &transaction,
            "https://marreq.example.test",
        )
        .await
        .unwrap();
        server.join().unwrap();

        assert_eq!(identity.issuer, "https://gitlab.example.test");
        assert_eq!(identity.subject, "987654");
        assert_eq!(identity.username.as_deref(), Some("alice"));
        assert!(identity.email_verified);
    }

    #[rocket::async_test]
    async fn oauth_exchange_rejects_an_incorrect_pkce_verifier() {
        let mut transaction = transaction();
        let expected_verifier = transaction.pkce_verifier.clone();
        transaction.pkce_verifier = "different-verifier".into();
        let (base, server) = mock_oauth_server(expected_verifier, false);

        let result = exchange_code(
            &provider(&base),
            "authorization-code",
            &transaction,
            "https://marreq.example.test",
        )
        .await;
        server.join().unwrap();

        assert_eq!(result.unwrap_err(), "provider token exchange failed");
    }

    #[rocket::async_test]
    async fn oidc_authorization_includes_nonce_state_and_s256_pkce() {
        let (issuer, server) = mock_oidc_discovery_server();
        let mut transaction = transaction();
        transaction.provider_key = "oidc".into();

        let authorization = authorization_url(
            &oidc_provider(&issuer),
            &transaction,
            "https://marreq.example.test",
        )
        .await
        .unwrap();
        server.join().unwrap();
        let url = url::Url::parse(&authorization).unwrap();
        let query = url
            .query_pairs()
            .collect::<std::collections::HashMap<_, _>>();

        assert_eq!(query.get("nonce").unwrap(), &transaction.nonce);
        assert_eq!(query.get("state").unwrap(), &transaction.state);
        assert_eq!(query.get("code_challenge_method").unwrap(), "S256");
        assert_eq!(
            query.get("code_challenge").unwrap(),
            &crate::auth::pkce_challenge(&transaction.pkce_verifier)
        );
    }

    #[test]
    fn oidc_discovery_issuer_must_match_exactly() {
        assert!(issuer_matches(
            "https://sso.example.test/realms/marreq",
            "https://sso.example.test/realms/marreq"
        ));
        assert!(!issuer_matches(
            "https://sso.example.test/realms/marreq",
            "https://evil.example.test/realms/marreq"
        ));
        assert!(!issuer_matches(
            "https://sso.example.test/realms/marreq",
            "https://sso.example.test/realms/marreq/"
        ));
    }

    fn mock_oauth_server(
        expected_verifier: String,
        accept_verifier: bool,
    ) -> (String, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (stream, request) = read_request(&listener);
            assert!(request.starts_with("POST /token "));
            assert!(request.contains("code=authorization-code"));
            assert_eq!(
                request.contains(&format!("code_verifier={expected_verifier}")),
                accept_verifier
            );
            if !accept_verifier {
                respond(stream, "400 Bad Request", r#"{"error":"invalid_grant"}"#);
                return;
            }
            respond(
                stream,
                "200 OK",
                r#"{"access_token":"temporary-token","token_type":"bearer"}"#,
            );

            let (stream, request) = read_request(&listener);
            assert!(request.starts_with("GET /user "));
            assert!(request
                .to_ascii_lowercase()
                .contains("authorization: bearer temporary-token"));
            respond(
                stream,
                "200 OK",
                r#"{"id":987654,"username":"alice","name":"Alice","email":"alice@example.test","confirmed_at":"2026-01-01T00:00:00Z"}"#,
            );
        });
        (format!("http://{address}"), server)
    }

    fn mock_oidc_discovery_server() -> (String, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let issuer = format!("http://{address}");
        let server_issuer = issuer.clone();
        let server = std::thread::spawn(move || {
            let (stream, request) = read_request(&listener);
            assert!(request.starts_with("GET /.well-known/openid-configuration "));
            let metadata = serde_json::json!({
                "issuer": server_issuer,
                "authorization_endpoint": format!("{server_issuer}/authorize"),
                "token_endpoint": format!("{server_issuer}/token"),
                "jwks_uri": format!("{server_issuer}/jwks"),
                "response_types_supported": ["code"],
                "subject_types_supported": ["public"],
                "id_token_signing_alg_values_supported": ["RS256"]
            })
            .to_string();
            respond(stream, "200 OK", &metadata);

            let (stream, request) = read_request(&listener);
            assert!(request.starts_with("GET /jwks "));
            respond(stream, "200 OK", r#"{"keys":[]}"#);
        });
        (issuer, server)
    }

    fn read_request(listener: &TcpListener) -> (TcpStream, String) {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        let mut bytes = Vec::new();
        let mut buffer = [0u8; 4096];
        loop {
            let count = stream.read(&mut buffer).unwrap();
            if count == 0 {
                break;
            }
            bytes.extend_from_slice(&buffer[..count]);
            if let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&bytes[..header_end]);
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        line.to_ascii_lowercase()
                            .strip_prefix("content-length: ")
                            .map(str::to_owned)
                    })
                    .and_then(|value| value.parse::<usize>().ok())
                    .unwrap_or(0);
                if bytes.len() >= header_end + 4 + content_length {
                    break;
                }
            }
        }
        (stream, String::from_utf8(bytes).unwrap())
    }

    fn respond(mut stream: TcpStream, status: &str, body: &str) {
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).unwrap();
    }
}
