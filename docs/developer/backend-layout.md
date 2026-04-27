# Backend layout (Rocket API)

The Marreq **Rust / Rocket application** lives in **`backend/`** at the repository root (`backend/Cargo.toml`, `backend/src/`, `backend/migrations/`, `backend/templates/` for legacy SSR). The repo root **`Cargo.toml`** is a virtual **workspace** with `members = ["backend"]` so you can run `cargo build`, `cargo test -p marreq`, etc. from the monorepo root.

Docker continues to use the **repository root** as build context with `docker/Dockerfile`.

## Deployment modes

Marreq is built in exactly one backend deployment mode:

- **Server** (default): self-hosted, administrator-managed users, no public registration.
- **Cloud** (`--no-default-features --features cloud`): hosted/SaaS mode with public registration, email verification, password reset, and one environment-bootstrapped site administrator.

Do not enable both `server` and `cloud`; the crate intentionally rejects that combination.

Cloud mode uses these environment variables:

| Variable | Purpose |
| --- | --- |
| `MARREQ_PUBLIC_BASE_URL` | Public SPA origin used in verification and password-reset links. Defaults to `http://localhost:8000`. |
| `MARREQ_SITE_ADMIN_EMAIL` | Email address of the single Cloud site administrator. Existing users are promoted and verified. |
| `MARREQ_SITE_ADMIN_BOOTSTRAP_PASSWORD` | Optional initial password used only when the Cloud site administrator does not already exist. |
| `SMTP_HOST`, `SMTP_PORT`, `SMTP_USERNAME`, `SMTP_PASSWORD`, `SMTP_FROM_ADDRESS` | SMTP settings for Cloud auth email and user notifications. |

The SPA can inspect `GET /api/meta/deployment` to decide whether to show self-service registration and password-reset UI.

## Split frontend/backend stack

The Rocket backend serves JSON under `/api`; the React/Vite frontend owns browser routes such as `/`, `/login`, `/:namespace/:projectSlug`, `/verify-email`, and `/reset-password`.

In Docker, nginx on **:8080** serves the Vite build and reverse-proxies `/api/` to the backend on **:8000**. Session cookies stay on one origin in that stack.

## API surface (high level)

- **Project reviewers**: `GET` / `PUT` `/api/projects/<project_id>/reviewers` (body `{"user_ids":[...]}`) — who may change requirement **status**, verification **status**, and requirement-version **approval** (plus global admins). Implemented with the members / project API module; see `doc/API.md` after changes land.
- **Deployment metadata**: `GET /api/meta/deployment` — current mode and capability flags for clients.
- **Cloud public auth**: `POST /api/auth/register`, `GET /api/auth/verify-email`, `POST /api/auth/forgot-password`, `POST /api/auth/reset-password` — only mounted in Cloud mode.

## Related docs

- [doc/API.md](../../doc/API.md) — interchangeable-frontend contract
- [docker/README.md](../../docker/README.md) — compose services `backend` + `frontend`
