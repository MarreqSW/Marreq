use rocket::form::{Form, FromForm};
use rocket::http::Status;
use rocket::response::content::RawHtml;
use rocket::response::Redirect;
use rocket::serde::json::{json, Json};

use crate::api::prelude::*;
use crate::auth::delegated::{self, AuthorizeRequest, DelegatedOAuthError, SCOPES};
use crate::auth::guards::SessionUser;
use crate::repository::DelegatedOAuthRepository;

fn issuer() -> String {
    crate::config::AppConfig::current()
        .public_base_url
        .trim_end_matches('/')
        .to_owned()
}
fn resource() -> String {
    format!("{}/mcp", issuer())
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
        json!({ "issuer": base, "authorization_endpoint": format!("{base}/oauth/authorize"), "token_endpoint": format!("{base}/oauth/token"), "registration_endpoint": format!("{base}/oauth/register"), "response_types_supported": ["code"], "grant_types_supported": ["authorization_code", "refresh_token"], "code_challenge_methods_supported": ["S256"], "scopes_supported": SCOPES, "token_endpoint_auth_methods_supported": ["none"] }),
    )
}

#[get("/.well-known/oauth-protected-resource")]
pub fn protected_resource_metadata() -> Json<serde_json::Value> {
    Json(
        json!({ "resource": resource(), "authorization_servers": [issuer()], "scopes_supported": SCOPES, "bearer_methods_supported": ["header"] }),
    )
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
) -> ApiResult<(Status, Json<serde_json::Value>)> {
    let mut repo = state
        .try_repo_write()
        .map_err(|_| ApiError::Internal("repository unavailable".into()))?;
    let client =
        delegated::register_client(&mut *repo, &body.client_name, body.redirect_uris.clone())
            .map_err(oauth_error)?;
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

#[get("/oauth/authorize?<query..>")]
pub fn authorize_page(
    query: AuthorizeQuery,
    user: SessionUser,
    state: &State<AppState>,
    cookies: &rocket::http::CookieJar<'_>,
) -> ApiResult<RawHtml<String>> {
    let repo = state
        .try_repo_read()
        .map_err(|_| ApiError::Internal("repository unavailable".into()))?;
    let (client, scopes) = checked_request(&*repo, &query)?;
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
    Ok(RawHtml(format!(
        r#"<!doctype html><html><head><meta charset="utf-8"><title>Authorize {name}</title></head><body><main><h1>Authorize {name}</h1><p>Signed in as <strong>{user}</strong></p><p>This application requests:</p><ul>{capabilities}</ul><form method="post" action="/oauth/authorize">{hidden}<input type="hidden" name="csrf" value="{csrf}"><button name="decision" value="allow">Allow</button><button name="decision" value="deny">Deny</button></form></main></body></html>"#,
        name = escape(&client.name),
        user = escape(&user.username),
        capabilities = capabilities,
        hidden = hidden,
        csrf = escape(&csrf)
    )))
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
            .append_pair("state", &form.state);
        return Ok(Redirect::to(url.to_string()));
    }
    let scopes = form.scope.split_whitespace().map(str::to_owned).collect();
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
    url.query_pairs_mut()
        .append_pair("code", &code)
        .append_pair("state", &form.state);
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
) -> ApiResult<Json<delegated::TokenResponse>> {
    let form = form.into_inner();
    let now = chrono::Utc::now().naive_utc();
    let mut repo = state
        .try_repo_write()
        .map_err(|_| ApiError::Internal("repository unavailable".into()))?;
    let response = match form.grant_type.as_str() {
        "authorization_code" => delegated::exchange_code(
            &mut *repo,
            form.code
                .as_deref()
                .ok_or_else(|| ApiError::BadRequest("invalid_request".into()))?,
            form.code_verifier
                .as_deref()
                .ok_or_else(|| ApiError::BadRequest("invalid_request".into()))?,
            &form.client_id,
            form.redirect_uri
                .as_deref()
                .ok_or_else(|| ApiError::BadRequest("invalid_request".into()))?,
            &form.resource,
            now,
        ),
        "refresh_token" => delegated::refresh(
            &mut *repo,
            form.refresh_token
                .as_deref()
                .ok_or_else(|| ApiError::BadRequest("invalid_request".into()))?,
            &form.client_id,
            &form.resource,
            now,
        ),
        _ => return Err(ApiError::BadRequest("unsupported_grant_type".into())),
    }
    .map_err(oauth_error)?;
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
        Ok(Status::NoContent)
    } else {
        Err(ApiError::NotFound("grant not found".into()))
    }
}

pub fn routes() -> Vec<rocket::Route> {
    routes![
        authorization_server_metadata,
        protected_resource_metadata,
        register,
        authorize_page,
        authorize_decision,
        token,
        grants,
        revoke_grant
    ]
}
