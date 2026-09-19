# Authentication and federated identity

Marreq uses one browser-authentication engine in `marreq-core`. Both deployment
binaries share session rotation, CSRF rotation, OAuth transaction validation,
identity resolution, provisioning, account linking, and audit behavior.
Bearer API tokens and MCP authentication are separate and unchanged.

## Delegated applications

Federated identities answer how a person signs in. Delegated OAuth grants are a
separate domain: they record which external application may act for that signed-in
user. Marreq never gives an external client an upstream Google, GitHub, GitLab,
or OIDC credential.

Marreq is an OAuth authorization server for the remote MCP resource. It supports
Authorization Code with mandatory PKCE S256 and rotating refresh tokens. The
browser authorization endpoint requires the existing Marreq session and displays
the application name, signed-in account, and requested scopes before Allow/Deny.
Account settings lists active applications and can revoke a grant immediately.

The access-token design is opaque: authorization codes, access tokens, and
refresh tokens are random 256-bit values, while PostgreSQL stores SHA-256 hashes.
Codes expire after five minutes and are single use; access tokens expire after
15 minutes; refresh tokens expire after 30 days and rotate on every use. Reuse
of a rotated refresh token revokes its token family. Grant revocation deletes
active access tokens and revokes all refresh tokens for the grant.

OAuth endpoints:

| Method | Path | Purpose |
|---|---|---|
| `GET` | `/.well-known/oauth-authorization-server` | Authorization-server metadata. |
| `GET` | `/.well-known/oauth-protected-resource` | MCP protected-resource metadata. |
| `POST` | `/oauth/register` | Register a public client and exact redirect URIs. |
| `GET` | `/oauth/authorize` | Validate the request and show consent. |
| `POST` | `/oauth/authorize` | Allow or deny using the authenticated browser session. |
| `POST` | `/oauth/token` | Exchange a code or rotate a refresh token. |
| `GET` | `/api/oauth/grants` | List the user's connected applications. |
| `DELETE` | `/api/oauth/grants/{id}` | Revoke an owned grant (CSRF protected). |

The MCP resource indicator is `<MARREQ_PUBLIC_BASE_URL>/mcp`. Redirect URIs use
exact matching; HTTPS is mandatory except for loopback localhost development.

Supported scopes are `projects:read`, `requirements:read`,
`requirements:write`, `requirements:approve`, `verifications:read`,
`verifications:write`, `traceability:read`, `traceability:write`,
`baselines:read`, and `baselines:write`. Scope is only a delegation ceiling:
the authenticated Marreq user's normal membership, role, reviewer, and project
authorization checks still run on every REST request.

## Identity model

Local passwords are optional. External identities are keyed only by the stable
pair `(issuer, subject)` in `user_identities`; email and provider usernames are
profile attributes and are never proof that two accounts are the same. Marreq
does not persist provider access tokens, refresh tokens, authorization codes, or
ID tokens.

An unknown external identity whose email is already present is rejected with an
account-link-required conflict. The user must sign in normally and connect the
provider from Account settings. Marreq also refuses to disconnect a user's last
usable authentication method.

## Browser flow

All providers use Authorization Code with PKCE S256. A private, HttpOnly,
SameSite=Lax cookie binds a random state, OIDC nonce, PKCE verifier, provider,
operation, expiry, and local `return_to` path to the initiating browser. The
cookie expires after ten minutes and is removed before callback validation, so
callbacks are single use. Production should set
`MARREQ_SECURE_SESSION_COOKIE=1`.

The callback URL is deterministic:

```text
<MARREQ_PUBLIC_BASE_URL>/api/auth/external/<provider>/callback
```

Only local paths such as `/projects` are accepted for `return_to`; absolute and
scheme-relative URLs are rejected.

## Self-hosted Server OIDC

```dotenv
MARREQ_PUBLIC_BASE_URL=https://marreq.example.com
MARREQ_OIDC_ENABLED=true
MARREQ_OIDC_DISPLAY_NAME="Company SSO"
MARREQ_OIDC_ISSUER=https://sso.example.com/realms/company
MARREQ_OIDC_CLIENT_ID=marreq
MARREQ_OIDC_CLIENT_SECRET=replace-me
MARREQ_OIDC_AUTO_REGISTER=false
```

Register `https://marreq.example.com/api/auth/external/oidc/callback` at the
provider. Discovery must return exactly the configured issuer. Signature, key,
issuer, audience, expiry, and nonce checks are performed by `openidconnect`.
Issuer URLs must use HTTPS, except `http://localhost` for local development.

The configuration works with standards-compliant discovery providers such as
Keycloak and Authentik without provider-specific code. Auto-registration is off
by default; pre-link identities or explicitly enable it when public provisioning
is intended.

## Marreq Cloud providers

Cloud supports Google OIDC plus GitHub and GitLab OAuth2. Set the matching
`MARREQ_<PROVIDER>_CLIENT_ID` and `MARREQ_<PROVIDER>_CLIENT_SECRET` pair to
enable a provider. Cloud automatically provisions unknown identities, including
their personal workspace. Google uses the OIDC `sub`; GitHub uses its immutable
numeric user ID with canonical issuer `https://github.com`; GitLab uses its
stable user ID with canonical issuer `https://gitlab.com`.

## Public API

| Method | Path | Purpose |
|---|---|---|
| `GET` | `/api/auth/providers` | Public enabled-method discovery; contains no credentials. |
| `GET` | `/api/auth/external/{provider}/start` | Begin a login redirect. |
| `GET` | `/api/auth/external/{provider}/callback` | Provider-specific callback. |
| `GET` | `/api/auth/identities` | List the signed-in user's connected methods. |
| `POST` | `/api/auth/external/{provider}/link` | Begin explicit linking; CSRF protected. |
| `DELETE` | `/api/auth/identities/{id}` | Disconnect an owned identity; CSRF protected. |

Provider secrets and protocol endpoints are backend-only. The frontend performs
no token exchange.
