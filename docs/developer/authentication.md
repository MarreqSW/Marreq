# Authentication and federated identity

Marreq uses one browser-authentication engine in `marreq-core`. Both deployment
binaries share session rotation, CSRF rotation, OAuth transaction validation,
identity resolution, provisioning, account linking, and audit behavior.
Bearer API tokens and MCP authentication are separate and unchanged.

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
