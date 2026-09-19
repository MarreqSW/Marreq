use rocket::form::{Form, FromForm};
use rocket::http::Status;
use rocket::response::content::RawHtml;
use rocket::response::Redirect;
use rocket::serde::json::{json, Json};
use rocket::{Request, Response};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::api::guards::OptionalSessionUser;
use crate::api::prelude::*;
use crate::auth::delegated::{self, AuthorizeRequest, DelegatedOAuthError, SCOPES};
use crate::auth::guards::SessionUser;
use crate::repository::DelegatedOAuthRepository;
use crate::repository::LogRepository;

fn issuer() -> String {
    crate::config::AppConfig::current()
        .public_base_url
        .trim_end_matches('/')
        .to_owned()
}
fn resource() -> String {
    crate::config::AppConfig::current()
        .mcp_public_url
        .trim_end_matches('/')
        .to_owned()
}
fn oauth_error(error: DelegatedOAuthError) -> ApiError {
    match error {
        DelegatedOAuthError::InvalidClient | DelegatedOAuthError::AccessDenied => {
            ApiError::Unauthorized(error.to_string())
        }
        DelegatedOAuthError::InvalidScope
        | DelegatedOAuthError::InvalidTarget
        | DelegatedOAuthError::InvalidGrant
        | DelegatedOAuthError::InvalidRequest => ApiError::BadRequest(error.to_string()),
        DelegatedOAuthError::Repository(_) => {
            ApiError::Internal("delegated authorization unavailable".into())
        }
    }
}

#[derive(Debug, serde::Serialize)]
#[serde(crate = "rocket::serde")]
pub struct OAuthProtocolError {
    error: String,
}
impl OAuthProtocolError {
    fn new(error: impl Into<String>) -> Self {
        Self {
            error: error.into(),
        }
    }
}
impl<'r> rocket::response::Responder<'r, 'static> for OAuthProtocolError {
    fn respond_to(self, request: &'r Request<'_>) -> rocket::response::Result<'static> {
        let status = match self.error.as_str() {
            "invalid_client" => Status::Unauthorized,
            "temporarily_unavailable" => Status::ServiceUnavailable,
            _ => Status::BadRequest,
        };
        Response::build_from(Json(self).respond_to(request)?)
            .status(status)
            .ok()
    }
}

pub struct OAuthRegistrationRateLimiter(Mutex<HashMap<IpAddr, Vec<Instant>>>);
impl OAuthRegistrationRateLimiter {
    pub fn new() -> Self {
        Self(Mutex::new(HashMap::new()))
    }
    fn allow(&self, ip: IpAddr) -> bool {
        let Ok(mut entries) = self.0.lock() else {
            return false;
        };
        let now = Instant::now();
        let attempts = entries.entry(ip).or_default();
        attempts.retain(|at| now.duration_since(*at) < Duration::from_secs(3600));
        if attempts.len() >= 20 {
            return false;
        }
        attempts.push(now);
        true
    }
}
impl Default for OAuthRegistrationRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}
fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[get("/.well-known/oauth-authorization-server")]
pub fn authorization_server_metadata() -> Json<serde_json::Value> {
    let base = issuer();
    Json(
        json!({ "issuer": base, "authorization_endpoint": format!("{base}/oauth/authorize"), "token_endpoint": format!("{base}/oauth/token"), "registration_endpoint": format!("{base}/oauth/register"), "response_types_supported": ["code"], "grant_types_supported": ["authorization_code", "refresh_token"], "code_challenge_methods_supported": ["S256"], "scopes_supported": SCOPES, "token_endpoint_auth_methods_supported": ["none"], "authorization_response_iss_parameter_supported": true }),
    )
}

#[get("/.well-known/oauth-protected-resource")]
pub fn protected_resource_metadata() -> Json<serde_json::Value> {
    Json(
        json!({ "resource": resource(), "authorization_servers": [issuer()], "scopes_supported": SCOPES, "bearer_methods_supported": ["header"] }),
    )
}

#[get("/.well-known/oauth-protected-resource/mcp")]
pub fn protected_resource_metadata_mcp() -> Json<serde_json::Value> {
    protected_resource_metadata()
}

#[derive(serde::Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct RegistrationRequest {
    client_name: String,
    redirect_uris: Vec<String>,
}

#[post("/oauth/register", format = "json", data = "<body>")]
pub fn register(
    body: Json<RegistrationRequest>,
    state: &State<AppState>,
    limiter: &State<OAuthRegistrationRateLimiter>,
    client_ip: Option<IpAddr>,
) -> Result<(Status, Json<serde_json::Value>), OAuthProtocolError> {
    if client_ip.is_some_and(|ip| !limiter.allow(ip)) {
        return Err(OAuthProtocolError::new("temporarily_unavailable"));
    }
    let mut repo = state
        .try_repo_write()
        .map_err(|_| OAuthProtocolError::new("temporarily_unavailable"))?;
    let client =
        delegated::register_client(&mut *repo, &body.client_name, body.redirect_uris.clone())
            .map_err(|error| OAuthProtocolError::new(error.to_string()))?;
    Ok((
        Status::Created,
        Json(
            json!({ "client_id": client.client_id, "client_name": client.name, "redirect_uris": client.redirect_uris, "token_endpoint_auth_method": "none" }),
        ),
    ))
}

#[derive(FromForm)]
pub struct AuthorizeQuery {
    response_type: String,
    client_id: String,
    redirect_uri: String,
    scope: String,
    state: String,
    code_challenge: String,
    code_challenge_method: String,
    resource: String,
}

fn checked_request<R: DelegatedOAuthRepository>(
    repo: &R,
    q: &AuthorizeQuery,
) -> Result<(crate::models::OAuthClient, Vec<String>), ApiError> {
    if q.response_type != "code" || q.code_challenge_method != "S256" || q.resource != resource() {
        return Err(ApiError::BadRequest("invalid_request".into()));
    }
    let client = repo
        .get_oauth_client(&q.client_id)
        .map_err(|_| ApiError::Unauthorized("invalid_client".into()))?;
    let redirects = client
        .redirect_uris
        .as_array()
        .ok_or_else(|| ApiError::BadRequest("invalid_client".into()))?;
    if !redirects
        .iter()
        .any(|v| v.as_str() == Some(q.redirect_uri.as_str()))
    {
        return Err(ApiError::BadRequest("invalid redirect_uri".into()));
    }
    let scopes = q
        .scope
        .split_whitespace()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if scopes.is_empty() || scopes.iter().any(|s| !SCOPES.contains(&s.as_str())) {
        return Err(ApiError::BadRequest("invalid_scope".into()));
    }
    Ok((client, scopes))
}

fn authorization_return_path(query: &AuthorizeQuery) -> String {
    let mut url =
        url::Url::parse("http://localhost/oauth/authorize").expect("fixed continuation URL");
    url.query_pairs_mut()
        .append_pair("response_type", &query.response_type)
        .append_pair("client_id", &query.client_id)
        .append_pair("redirect_uri", &query.redirect_uri)
        .append_pair("scope", &query.scope)
        .append_pair("state", &query.state)
        .append_pair("code_challenge", &query.code_challenge)
        .append_pair("code_challenge_method", &query.code_challenge_method)
        .append_pair("resource", &query.resource);
    format!("{}?{}", url.path(), url.query().unwrap_or_default())
}

#[get("/oauth/authorize?<query..>")]
pub fn authorize_page(
    query: AuthorizeQuery,
    user: OptionalSessionUser,
    state: &State<AppState>,
    cookies: &rocket::http::CookieJar<'_>,
) -> ApiResult<AuthorizePageResponse> {
    let repo = state
        .try_repo_read()
        .map_err(|_| ApiError::Internal("repository unavailable".into()))?;
    let (client, scopes) = match checked_request(&*repo, &query) {
        Ok(value) => value,
        Err(error) => {
            let trusted_redirect = repo
                .get_oauth_client(&query.client_id)
                .ok()
                .and_then(|client| client.redirect_uris.as_array().cloned())
                .is_some_and(|redirects| {
                    redirects
                        .iter()
                        .any(|value| value.as_str() == Some(query.redirect_uri.as_str()))
                });
            if !trusted_redirect {
                return Err(error);
            }
            let mut url = url::Url::parse(&query.redirect_uri)
                .map_err(|_| ApiError::BadRequest("invalid redirect_uri".into()))?;
            let code = match error.message() {
                "invalid_scope" => "invalid_scope",
                _ => "invalid_request",
            };
            url.query_pairs_mut()
                .append_pair("error", code)
                .append_pair("state", &query.state)
                .append_pair("iss", &issuer());
            return Ok(AuthorizePageResponse::Login(Box::new(Redirect::to(
                url.to_string(),
            ))));
        }
    };
    let return_to = authorization_return_path(&query);
    let Some(user) = user.0 else {
        cookies.add_private(
            rocket::http::Cookie::build(("oauth_authorize_pending", return_to.clone()))
                .http_only(true)
                .same_site(rocket::http::SameSite::Lax)
                .max_age(rocket::time::Duration::minutes(10))
                .build(),
        );
        return Ok(AuthorizePageResponse::Login(Box::new(Redirect::to(
            format!("/login?return_to={}", urlencoding::encode(&return_to)),
        ))));
    };
    if let Some(pending) = cookies.get_private("oauth_authorize_pending") {
        if pending.value() != return_to {
            return Err(ApiError::BadRequest(
                "authorization transaction changed during login".into(),
            ));
        }
        cookies.remove_private(rocket::http::Cookie::from("oauth_authorize_pending"));
    }
    let csrf = crate::auth::csrf::get_or_create_csrf_token(cookies);
    let hidden = [
        ("client_id", query.client_id.as_str()),
        ("redirect_uri", query.redirect_uri.as_str()),
        ("scope", query.scope.as_str()),
        ("state", query.state.as_str()),
        ("code_challenge", query.code_challenge.as_str()),
        ("resource", query.resource.as_str()),
    ]
    .iter()
    .map(|(n, v)| {
        format!(
            r#"<input type="hidden" name="{}" value="{}">"#,
            n,
            escape(v)
        )
    })
    .collect::<String>();
    let capabilities = scopes
        .iter()
        .map(|s| format!("<li>{}</li>", escape(s)))
        .collect::<String>();
    Ok(AuthorizePageResponse::Consent(RawHtml(format!(
        r#"<!doctype html><html><head><meta charset="utf-8"><title>Authorize {name}</title></head><body><main><h1>Authorize {name}</h1><p>This name is registered metadata; the application's identity has not been independently verified.</p><p>Signed in as <strong>{user}</strong></p><p>This application requests:</p><ul>{capabilities}</ul><form method="post" action="/oauth/authorize">{hidden}<input type="hidden" name="csrf" value="{csrf}"><button name="decision" value="allow">Allow</button><button name="decision" value="deny">Deny</button></form></main></body></html>"#,
        name = escape(&client.name),
        user = escape(&user.username),
        capabilities = capabilities,
        hidden = hidden,
        csrf = escape(&csrf)
    ))))
}

#[derive(rocket::response::Responder)]
pub enum AuthorizePageResponse {
    Consent(RawHtml<String>),
    Login(Box<Redirect>),
}

#[derive(FromForm)]
pub struct ConsentForm {
    client_id: String,
    redirect_uri: String,
    scope: String,
    state: String,
    code_challenge: String,
    resource: String,
    decision: String,
}

#[post("/oauth/authorize", data = "<form>")]
pub fn authorize_decision(
    form: Form<ConsentForm>,
    user: SessionUser,
    state: &State<AppState>,
) -> ApiResult<Redirect> {
    let form = form.into_inner();
    let query = AuthorizeQuery {
        response_type: "code".into(),
        client_id: form.client_id.clone(),
        redirect_uri: form.redirect_uri.clone(),
        scope: form.scope.clone(),
        state: form.state.clone(),
        code_challenge: form.code_challenge.clone(),
        code_challenge_method: "S256".into(),
        resource: form.resource.clone(),
    };
    {
        let repo = state
            .try_repo_read()
            .map_err(|_| ApiError::Internal("repository unavailable".into()))?;
        checked_request(&*repo, &query)?;
    }
    let mut url = url::Url::parse(&form.redirect_uri)
        .map_err(|_| ApiError::BadRequest("invalid redirect_uri".into()))?;
    if form.decision != "allow" {
        url.query_pairs_mut()
            .append_pair("error", "access_denied")
            .append_pair("state", &form.state)
            .append_pair("iss", &issuer());
        return Ok(Redirect::to(url.to_string()));
    }
    let scopes = form.scope.split_whitespace().map(str::to_owned).collect();
    let audit_client_id = form.client_id.clone();
    let audit_resource = form.resource.clone();
    let mut repo = state
        .try_repo_write()
        .map_err(|_| ApiError::Internal("repository unavailable".into()))?;
    let code = delegated::authorize(
        &mut *repo,
        user.id,
        AuthorizeRequest {
            client_id: form.client_id,
            redirect_uri: form.redirect_uri,
            scopes,
            resource: form.resource,
            code_challenge: form.code_challenge,
        },
        &resource(),
        chrono::Utc::now().naive_utc(),
    )
    .map_err(oauth_error)?;
    let _ = repo.insert_log(&crate::models::NewLog {
        user_id: user.id,
        action_type: "OAUTH_GRANT_CREATED".into(),
        entity_type: "OAUTH_GRANT".into(),
        entity_id: None,
        project_id: None,
        old_values: None,
        new_values: Some(json!({"client_id": audit_client_id, "scopes": form.scope.split_whitespace().collect::<Vec<_>>(), "resource": audit_resource}).to_string()),
        description: Some("Delegated application authorized".into()),
        ip_address: None,
        user_agent: None,
    });
    url.query_pairs_mut()
        .append_pair("code", &code)
        .append_pair("state", &form.state)
        .append_pair("iss", &issuer());
    Ok(Redirect::to(url.to_string()))
}

#[derive(FromForm)]
pub struct TokenForm {
    grant_type: String,
    code: Option<String>,
    redirect_uri: Option<String>,
    client_id: String,
    code_verifier: Option<String>,
    refresh_token: Option<String>,
    resource: String,
}

#[post("/oauth/token", data = "<form>")]
pub fn token(
    form: Form<TokenForm>,
    state: &State<AppState>,
) -> Result<Json<delegated::TokenResponse>, OAuthProtocolError> {
    let form = form.into_inner();
    let now = chrono::Utc::now().naive_utc();
    let mut repo = state
        .try_repo_write()
        .map_err(|_| OAuthProtocolError::new("temporarily_unavailable"))?;
    let response = match form.grant_type.as_str() {
        "authorization_code" => delegated::exchange_code(
            &mut *repo,
            form.code
                .as_deref()
                .ok_or_else(|| OAuthProtocolError::new("invalid_request"))?,
            form.code_verifier
                .as_deref()
                .ok_or_else(|| OAuthProtocolError::new("invalid_request"))?,
            &form.client_id,
            form.redirect_uri
                .as_deref()
                .ok_or_else(|| OAuthProtocolError::new("invalid_request"))?,
            &form.resource,
            now,
        ),
        "refresh_token" => delegated::refresh(
            &mut *repo,
            form.refresh_token
                .as_deref()
                .ok_or_else(|| OAuthProtocolError::new("invalid_request"))?,
            &form.client_id,
            &form.resource,
            now,
        ),
        _ => return Err(OAuthProtocolError::new("unsupported_grant_type")),
    }
    .map_err(|error| OAuthProtocolError::new(error.to_string()))?;
    Ok(Json(response))
}

#[get("/api/oauth/grants")]
pub fn grants(user: SessionUser, state: &State<AppState>) -> ApiResult<Json<serde_json::Value>> {
    let repo = state
        .try_repo_read()
        .map_err(|_| ApiError::Internal("repository unavailable".into()))?;
    let rows = repo
        .list_oauth_grants(user.id)
        .map_err(|_| ApiError::Internal("repository unavailable".into()))?;
    Ok(Json(
        json!({ "grants": rows.into_iter().map(|(g,c)| json!({"id":g.id,"application_name":c.name,"scopes":g.scopes,"created_at":g.created_at,"last_used_at":g.last_used_at})).collect::<Vec<_>>() }),
    ))
}

#[delete("/api/oauth/grants/<id>")]
pub fn revoke_grant(id: i32, user: SessionUser, state: &State<AppState>) -> ApiResult<Status> {
    let mut repo = state
        .try_repo_write()
        .map_err(|_| ApiError::Internal("repository unavailable".into()))?;
    if repo
        .revoke_oauth_grant(id, user.id, chrono::Utc::now().naive_utc())
        .map_err(|_| ApiError::Internal("repository unavailable".into()))?
    {
        let _ = repo.insert_log(&crate::models::NewLog {
            user_id: user.id,
            action_type: "OAUTH_GRANT_REVOKED".into(),
            entity_type: "OAUTH_GRANT".into(),
            entity_id: Some(id),
            project_id: None,
            old_values: None,
            new_values: None,
            description: Some("Delegated application revoked".into()),
            ip_address: None,
            user_agent: None,
        });
        Ok(Status::NoContent)
    } else {
        Err(ApiError::NotFound("grant not found".into()))
    }
}

pub fn routes() -> Vec<rocket::Route> {
    routes![
        authorization_server_metadata,
        protected_resource_metadata,
        protected_resource_metadata_mcp,
        register,
        authorize_page,
        authorize_decision,
        token,
        grants,
        revoke_grant
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_continuation_preserves_the_complete_authorization_request() {
        let query = AuthorizeQuery {
            response_type: "code".into(),
            client_id: "client".into(),
            redirect_uri: "https://client.example/callback".into(),
            scope: "projects:read requirements:read".into(),
            state: "opaque-state".into(),
            code_challenge: "a".repeat(43),
            code_challenge_method: "S256".into(),
            resource: "https://marreq.example/mcp".into(),
        };
        let url = url::Url::parse(&format!(
            "http://localhost{}",
            authorization_return_path(&query)
        ))
        .unwrap();
        let values = url
            .query_pairs()
            .collect::<std::collections::HashMap<_, _>>();
        assert_eq!(values.get("client_id").map(|v| v.as_ref()), Some("client"));
        assert_eq!(
            values.get("state").map(|v| v.as_ref()),
            Some("opaque-state")
        );
        assert_eq!(
            values.get("redirect_uri").map(|v| v.as_ref()),
            Some("https://client.example/callback")
        );
        assert_eq!(
            values.get("resource").map(|v| v.as_ref()),
            Some("https://marreq.example/mcp")
        );
        assert_eq!(values.get("code_challenge").map(|v| v.len()), Some(43));
    }
}
