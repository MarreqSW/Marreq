# Installation guide (DevOps)

This document describes how to provision hosts and CI runners so you can build, test, and run **Marreq** (Rust backend, PostgreSQL with pgvector, React/Vite frontend, optional Ollama and MCP). For day-to-day developer workflows and deployment modes, see [docs/developer/setup.md](docs/developer/setup.md), [docs/developer/database-setup.md](docs/developer/database-setup.md), and [docker/README.md](docker/README.md).

## 1. What you are installing

| Layer | Technology | Notes |
|--------|------------|--------|
| Backend | Rust (stable), Cargo workspace | Binaries: `marreq-server` (self-hosted), `marreq-cloud` (SaaS-style) |
| Database | PostgreSQL **15** with **pgvector** | Compose uses `pgvector/pgvector:pg15-trixie` |
| Frontend | Node.js **20**, npm | Matches GitHub Actions; SPA under `frontend/` |
| Containers | Docker Engine + Compose plugin | Stack defined in `docker/docker-compose.yml` |
| Native build deps | **clang** / **libclang** | Required for `xlsxwriter` (bindgen); CI installs `libclang-dev` on Ubuntu |
| Migrations | **diesel_cli** (PostgreSQL) | Installed via `cargo install` |
| Optional: embeddings | **Ollama** | Compose service or host install; see [docs/developer/ollama-setup.md](docs/developer/ollama-setup.md) |
| Optional: MCP | Node **≥ 18** | Package in `mcp-server/` |

Root `package.json` holds workspace-level JS tooling (Vitest, Playwright, stylelint, PurgeCSS). The SPA lives in `frontend/package.json`.

## 2. Minimum versions (alignment with CI)

- **Rust**: `stable` with `rustfmt` and `clippy` (see [.github/workflows/marreq-ci.yml](.github/workflows/marreq-ci.yml)).
- **Node.js**: **20.x** (lint job and frontend jobs use `actions/setup-node` with `node-version: '20'`).
- **Docker**: recent Engine and Compose v2 (`docker compose`).
- **PostgreSQL**: 15 with pgvector extension when not using the bundled image.

## 3. Linux (Ubuntu/Debian) — host packages

Example for building and testing on the runner or a bare metal server:

```bash
sudo apt-get update
sudo apt-get install -y build-essential pkg-config cmake libclang-dev
```

Install Docker following [Docker’s official docs](https://docs.docker.com/engine/install/) for your distribution. Ensure your deployment user can run `docker` without sudo if that is your policy.

Install Rust (if not only using Docker for builds):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
rustup toolchain install stable
rustup component add rustfmt clippy
```

Install Node.js 20 (e.g. via [NodeSource](https://github.com/nodesource/distributions), `fnm`, or `nvm`):

```bash
# Example with nvm:
nvm install 20
nvm use 20
```

Install Diesel CLI (needed for migrations outside the backend container entrypoint):

```bash
cargo install diesel_cli --no-default-features --features postgres
```

## 4. Configuration file

From the repository root:

```bash
cp .env.example .env
```

Edit `.env` for your environment: `DATABASE_URL`, `ROCKET_SECRET_KEY` (generate with `openssl rand -base64 32` for non-local deployments), optional `CSRF_ALLOWED_ORIGINS`, embeddings (`EMBEDDINGS_ENABLED`, `OLLAMA_URL`), cloud mode (`MARREQ_PUBLIC_BASE_URL`, `MARREQ_SITE_ADMIN_*`, `SMTP_*`). See comments inside [.env.example](.env.example).

## 5. Database

- **Docker (recommended for parity with compose):** host maps Postgres to **127.0.0.1:5433** by default (`DATABASE_URL=postgres://rust:rust@127.0.0.1:5433/marreq`).
- **Managed Postgres:** provision PostgreSQL 15, enable **pgvector**, create database and user, set `DATABASE_URL` accordingly.

Initialize schema (migrations):

```bash
./marreq-core/scripts/db_setup.sh
```

Optional demo data:

```bash
./marreq-core/scripts/db_setup.sh --seed
```

For reset, backups, and manual Diesel commands, see [docs/developer/database-setup.md](docs/developer/database-setup.md).

## 6. Docker stack (production-like local or server)

Build context is the repo root. Images are defined in `docker/Dockerfile` (backend) and `docker/frontend/Dockerfile` (nginx + Vite build).

Start only PostgreSQL:

```bash
docker compose -f docker/docker-compose.yml up -d db
```

Full self-hosted stack (database, Ollama, `marreq-server`, SPA on port **8080**, Adminer on **8081**):

```bash
docker compose -f docker/docker-compose.yml up -d
```

Cloud profile (`marreq-cloud` on **8001**, SPA on **8082**):

```bash
docker compose -f docker/docker-compose.yml --profile cloud up -d marreq-cloud frontend-cloud
```

Service ports and env expectations are documented in [docker/README.md](docker/README.md). Backend images run migrations at startup via `docker/docker-entrypoint.sh`.

Manual image builds:

```bash
docker build -f docker/Dockerfile -t marreq-server:local .
docker build -f docker/Dockerfile --build-arg MARREQ_BIN=marreq-cloud -t marreq-cloud:local .
docker build -f docker/frontend/Dockerfile -t marreq-frontend:local ..
```

## 7. Build without Docker (binaries on the host)

System packages: `build-essential`, `cmake`, `pkg-config`, `libclang-dev` (same family as the Dockerfile).

```bash
cargo build --workspace --release
cargo test --workspace
```

Run:

```bash
cargo run -p marreq-server
# or
cargo run -p marreq-cloud
```

Default Rocket URL for local API: **http://127.0.0.1:8000** (`/api`).

## 8. Frontend (SPA)

```bash
cd frontend
npm ci
npm run build    # production assets
npm run dev      # development with Vite (typically http://localhost:5173)
```

Root-level `npm ci` installs repo-wide test and lint tooling used in CI.

## 9. Optional: Ollama (semantic search)

Compose includes an `ollama` service (host port **11435** mapped to container **11434**). For host-installed Ollama and model pulls (`nomic-embed-text`, etc.), see [docs/developer/ollama-setup.md](docs/developer/ollama-setup.md). Set `EMBEDDINGS_ENABLED` and `OLLAMA_URL` in `.env` to match your deployment.

## 10. Optional: MCP server

For AI assistant integration against the REST API:

```bash
cd mcp-server
npm ci
npm run build
npm start
```

Requires a Marreq API base URL and Bearer token configuration as described in [docs/developer/mcp-setup.md](docs/developer/mcp-setup.md). Node **≥ 18** per `mcp-server/package.json`.

## 11. CI parity checklist

To mirror [.github/workflows/marreq-ci.yml](.github/workflows/marreq-ci.yml):

1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. Node 20: root `npm ci` → `npm test`; `frontend/` → `npm ci` → `npm run build`
4. Stylelint / PurgeCSS as in the workflow (root devDependencies)
5. Start DB: `docker compose -f docker/docker-compose.yml -f docker/docker-compose.ci.yml up -d db`
6. `DATABASE_URL=postgres://rust:rust@127.0.0.1:5433/marreq` → `./marreq-core/scripts/db_setup.sh`
7. `cargo install diesel_cli --no-default-features --features postgres` (if not already present)
8. `cargo build --workspace --release`, `cargo test --workspace`, coverage optional (`cargo-llvm-cov`)

Convenience script for local “all checks”: `bash marreq-core/scripts/run_checks.sh`.

## 12. Ports reference (default compose)

| Service | Host port |
|---------|-----------|
| PostgreSQL | 127.0.0.1:**5433** |
| `marreq-server` API | 127.0.0.1:**8000** |
| SPA (self-hosted nginx) | **8080** |
| Adminer | **8081** |
| `marreq-cloud` API | 127.0.0.1:**8001** |
| SPA (cloud nginx) | **8082** |
| Ollama (mapped) | **11435** |

Adjust firewall and reverse proxy rules accordingly; ensure `CSRF_ALLOWED_ORIGINS` matches the browser origin of the SPA.

## 13. Further reading

- [README.md](README.md) — overview and quick start
- [docs/developer/setup.md](docs/developer/setup.md) — server vs cloud, Docker vs native
- [docs/developer/http-api-contract.md](docs/developer/http-api-contract.md) — same-origin and proxy expectations
- [Makefile](Makefile) — shortcuts (`make compose-server`, `make docker-server`, etc.)
