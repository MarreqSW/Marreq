# Workspace layout & deployment modes

## Overview

The Marreq backend is split into three Rust crates that live as
**directories** inside the `Marreq` root repository (a single Cargo
workspace, no git submodules):

| Crate | Kind | Purpose |
|---|---|---|
| `marreq-core` | library | Shared domain, persistence (Diesel), Rocket primitives, auth, services, routes, fairings. Used by both deployment binaries. |
| `marreq-server` | binary | Self-hosted deployment. Admin-managed users; no public registration. |
| `marreq-cloud` | binary | Hosted / SaaS deployment. Self-registration, email verification, password reset, single bootstrap site administrator. |

The root `Marreq` repo is a **virtual Cargo workspace** — it contains no Rust
source of its own. It hosts the frontend, Docker files, documentation,
`mcp-server`, and developer tooling. A single top-level `Cargo.lock` keeps
dependency versions consistent across all three crates.

Deployment mode is selected at **compile time** by choosing which binary to
build, rather than by flipping a Cargo feature flag. No `--features
server`/`--features cloud` flag exists any more.

---

## Repository structure

```
Marreq/                         ← root repo (virtual workspace)
├── Cargo.toml                  ← workspace: ["marreq-core","marreq-server","marreq-cloud"]
├── Cargo.lock                  ← single lock-file for the whole workspace
├── Makefile                    ← convenience targets (see below)
├── marreq-core/                ← in-tree crate (library)
│   ├── Cargo.toml              ← [lib] marreq-core
│   ├── src/                    ← shared library source
│   ├── migrations/             ← Diesel migrations (schema source of truth)
│   ├── diesel.toml
│   └── scripts/                ← dev tooling & DB helpers (formerly backend/scripts/)
│       ├── db_setup.sh
│       ├── db_seed.sh
│       ├── db_migrate.sh
│       ├── db_reset.sh
│       ├── db_backup.sh
│       ├── run_checks.sh
│       ├── run_tests.sh
│       ├── run_ci.sh
│       └── init_complete.sql
├── marreq-server/              ← in-tree crate (binary)
│   ├── Cargo.toml              ← [[bin]] marreq-server; depends on marreq-core
│   └── src/
│       ├── main.rs             ← Rocket launch, wires Server deployment mode
│       ├── deployment.rs       ← impl DeploymentMode for Server
│       ├── api/                ← server-only routes (admin user management)
│       └── routes.rs           ← pub fn routes() → server-only route vec
├── marreq-cloud/               ← in-tree crate (binary)
│   ├── Cargo.toml              ← [[bin]] marreq-cloud; depends on marreq-core
│   └── src/
│       ├── main.rs             ← Rocket launch, wires Cloud deployment mode
│       ├── deployment.rs       ← impl DeploymentMode for Cloud
│       ├── api/auth_public.rs  ← register, verify-email, forgot/reset-password
│       ├── services/           ← registration_service.rs (cloud-only)
│       ├── fairings/           ← cloud_admin_bootstrap.rs
│       └── routes.rs           ← pub fn routes() → cloud-only route vec
├── frontend/                   ← React 19 + Vite SPA
├── docker/                     ← Dockerfile, docker-compose.yml, nginx config
├── docs/                       ← documentation (you are here)
└── mcp-server/                 ← optional Node/TypeScript MCP server for AI assistants
```

### Workspace `Cargo.toml` (root)

```toml
[workspace]
resolver = "2"
members = ["marreq-core", "marreq-server", "marreq-cloud"]

[workspace.package]
edition = "2021"
license = "AGPL-3.0-or-later"
```

Each deployment crate declares its library dependency as:

```toml
[dependencies]
marreq-core = { path = "../marreq-core" }
```

---

## Getting started

### Clone (fresh machine)

```bash
git clone git@github.com:MarreqSW/Marreq.git
cd Marreq
```

### Build the whole workspace

```bash
cargo build --workspace
```

### Run a deployment binary

```bash
# Self-hosted mode (default for most dev work)
cargo run -p marreq-server

# SaaS / hosted mode
cargo run -p marreq-cloud
```

### Makefile shortcuts

A top-level `Makefile` wraps the most common commands:

| Target | Command |
|---|---|
| `make server` | `cargo run -p marreq-server` |
| `make cloud` | `cargo run -p marreq-cloud` |
| `make build` | `cargo build --workspace --release` |
| `make test` | `cargo test --workspace` |
| `make fmt` | `cargo fmt --all` |
| `make lint` | `cargo clippy --workspace --all-targets -- -D warnings` |
| `make docker-server` | Build Docker image for marreq-server |
| `make docker-cloud` | Build Docker image for marreq-cloud |
| `make frontend` | `cd frontend && npm run dev` |

Run `make help` to list all targets with descriptions.

---

## Deployment modes

Deployment mode is chosen at **compile time** by selecting the binary crate —
no runtime flags or Cargo features need to be toggled:

| Behaviour | `marreq-server` | `marreq-cloud` |
|---|---|---|
| User creation | Admin-only (`POST /api/users`) | Public self-registration |
| Email verification | Not required | Required on sign-up |
| Password reset | Admin resets via API | Self-service via email link |
| Site admin bootstrap | N/A | `MARREQ_SITE_ADMIN_EMAIL` env var |
| Public auth routes | Not mounted | `/api/auth/register`, `/api/auth/forgot-password`, etc. |
| `GET /api/meta/deployment` | `{ "mode": "server" }` | `{ "mode": "cloud" }` |

### Cloud-mode environment variables

| Variable | Purpose |
|---|---|
| `MARREQ_PUBLIC_BASE_URL` | Public SPA origin for email links. Default: `http://localhost:8000`. |
| `MARREQ_SITE_ADMIN_EMAIL` | Email of the Cloud site admin. Existing users are promoted and verified. |
| `MARREQ_SITE_ADMIN_BOOTSTRAP_PASSWORD` | Initial password used only when the Cloud admin does not yet exist. |
| `SMTP_HOST`, `SMTP_PORT`, `SMTP_USERNAME`, `SMTP_PASSWORD`, `SMTP_FROM_ADDRESS` | SMTP settings for Cloud auth emails and notifications. |

The SPA reads `GET /api/meta/deployment` to decide whether to show the
self-service registration and password-reset UI.

---

## How to add a new shared module

1. Add source under `marreq-core/src/<module>/`.
2. Expose it in `marreq-core/src/lib.rs` (`pub mod <module>;` or
   `pub use <module>::…`).
3. Both deployment binaries gain access immediately via `marreq_core::<module>::…`.
4. If the module needs dependencies not yet in `marreq-core/Cargo.toml`, add
   them there. New shared deps go into the root `[workspace.dependencies]`
   block first; member crates reference them via `dep = { workspace = true }`.
5. If `marreq_core::deployment::current()` is read from the new module,
   remember that **every** binary must register a mode at startup via
   `app::build_with`. There is no fallback `default_mode()` any more —
   tests use `marreq_core::deployment::install_test_server_mode()` from the
   `test-helpers` feature.
6. Commit the change normally — all three crates live in the same repo.

---

## How to add a server-only or cloud-only feature

### Server-only

1. Add source under `marreq-server/src/` (e.g. `src/api/my_feature.rs`).
2. Register routes or fairings in `marreq-server/src/routes.rs`
   (`pub fn routes()`) or the fairing list in `main.rs`.
3. Commit normally — all three crates live in the same repo.

### Cloud-only

1. Add source under `marreq-cloud/src/` (e.g. `src/api/my_feature.rs`).
2. Register in `marreq-cloud/src/routes.rs` or the fairing list in `main.rs`.
3. Commit normally — all three crates live in the same repo.

The `DeploymentMode` trait lives in `marreq-core::deployment`. Each binary
has its own `impl DeploymentMode` in `src/deployment.rs`. Extend the trait
there if you need core to call back into deployment-specific logic.

---

## CI and Docker

- **GitHub Actions**: workflows in `.github/workflows/` build and test all
  three crates and run the frontend test suite. The CI matrix covers both
  `marreq-server` and `marreq-cloud` binaries.
- **Docker**: see [`docker/README.md`](../../docker/README.md). The default
  `docker/docker-compose.yml` builds and runs `marreq-server`. A separate
  `docker/Dockerfile.cloud` target produces the `marreq-cloud` image.
  The `docker-entrypoint.sh` applies Diesel migrations before launch.

---

## Database & scripts

Migrations and dev-helper scripts live inside the `marreq-core` crate:

- **Schema migrations**: `marreq-core/migrations/*/up.sql` (Diesel, shared by
  both binaries).
- **Diesel config**: `marreq-core/diesel.toml`.
- **Helper scripts**: `marreq-core/scripts/` — `db_setup.sh`, `db_seed.sh`,
  `db_migrate.sh`, `db_reset.sh`, `db_backup.sh`, `run_checks.sh`,
  `run_tests.sh`, `run_ci.sh`, `init_complete.sql`.

See the [database setup guide](database-setup.md) for full usage.

---

## Migration note

The legacy `backend/` single-crate layout was retired on **2026-04-27** as
part of the 3-crate workspace restructure. The old tree is still reachable
in the git history of the root repository. Cargo features `--features
server` and `--features cloud` no longer exist; choose the binary
(`-p marreq-server` or `-p marreq-cloud`) instead.

Follow-up cleanup landed shortly after retirement:

- `marreq_core::app::build()` and `marreq_core::deployment::default_mode()`
  (the legacy fallbacks kept alive while `backend/` was being torn down)
  were removed. Each binary must call `marreq_core::app::build_with(mode,
  routes, fairings)` at startup; tests use
  `marreq_core::deployment::install_test_server_mode()` (gated on the
  `test-helpers` Cargo feature).
- All shared dependencies now live in the root `[workspace.dependencies]`
  block; the three member `Cargo.toml` files reference them via
  `dep = { workspace = true }`.
- The Docker `backend` compose service was renamed to `marreq-server`, and
  a sibling `marreq-cloud` service is available behind the `cloud` profile
  (`docker compose --profile cloud up -d marreq-cloud`).

---

## Related docs

- [Database setup guide](database-setup.md)
- [Docker / Compose reference](../../docker/README.md)
- [HTTP API contract](../../doc/API.md)
- [MCP server setup](mcp-setup.md)
- [Semantic search / AI setup](semantic-search.md)
