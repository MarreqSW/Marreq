# Docker Layout and Usage

All container-related files live in this directory.

Copy the project env template before first use:

```bash
cp .env.example .env
```

## Files

- `docker-compose.yml`: Primary local stack — **`db`**, **`ollama`**, **`backend`** (Rocket API), **`frontend`** (nginx + SPA), **`adminer`**
- `docker-compose.dev.yml`: Developer override for running Marreq via `cargo run` inside Docker (`marreq-dev` on host port **8000**)
- `docker-compose.ci.yml`: CI-specific compose overrides
- `Dockerfile`: **Backend** image (Rust binary; build context: repository root). Accepts `MARREQ_BIN` build-arg (`marreq-server` by default; `marreq-cloud` for the cloud variant).
- `frontend/Dockerfile`: **Frontend** image (multi-stage: `npm run build` + nginx)
- `frontend/nginx.conf`: SPA on `/` only; `/api/` + legacy SSR paths (`/p/`, `/static/`, `/user/`, `/admin`, …) reverse-proxied to `backend:8000` when using hybrid mode (`MARREQ_DOCKER_SSR_PROXY=1`)
- `Dockerfile.dockerignore`: Build context exclusions for `Dockerfile`
- `docker-entrypoint.sh`: Backend container startup (wait for DB + migrations + start app)

## Split stack (default compose)

The default `docker-compose.yml` uses a **hybrid split stack**: Vite SPA for **`/`** only; nginx proxies legacy SSR paths (`/p/…`, `/static/…`, `/projects`, …) to Rocket so links work on **:8080**.

| Service   | Role |
|-----------|------|
| `db` | PostgreSQL published on host **`127.0.0.1:5433`** → container `5432` (avoids conflict with a local Postgres on **5432**). From the host, use `DATABASE_URL=postgres://rust:rust@127.0.0.1:5433/marreq` for `diesel`/scripts. |
| `ollama` | Published on host **`127.0.0.1:11435`** → container `11434` (avoids conflict with a local Ollama on **11434**). **`backend`** still uses `http://ollama:11434` on the Docker network. |
| `backend` | Rocket on **`127.0.0.1:8000`**: **`/api`**, plus HTML + **`/static`** when **`MARREQ_DOCKER_SSR_PROXY=1`**. **`MARREQ_UI_MODE`** empty; **`MARREQ_SERVE_STATIC=1`**. **`GET /`** on :8000 is the classic dashboard; use **:8080/** for the SPA. **`ROCKET_SECRET_KEY`**: compose default if missing. |
| `frontend` | Nginx: SPA for **`/`**; **`/api/`**, **`/static/`**, **`/p/`**, **`/user/`**, **`/admin`**, **`/projects`**, **`/logs`**, … → **`backend:8000`**. |
| `adminer` | Database UI on host **http://localhost:8081** (avoids clashing with frontend **8080**). |

Use the UI at **http://localhost:8080** so session cookies stay on the same origin as `/api`.

## Common Commands (from repo root)

Start only the database:

```bash
docker compose -f docker/docker-compose.yml up -d db
```

Start the full stack (db, ollama, backend, frontend, adminer):

```bash
docker compose -f docker/docker-compose.yml up -d
```

The Docker Compose files load the project `../.env` for shared app settings.
Docker-specific connection values such as the in-container `DATABASE_URL` and
`OLLAMA_URL` stay in the Compose files so they do not duplicate host-local
values in `.env`.

Start a Docker-only developer loop (Rocket **with** classic HTML UI on port 8000 — not the split SPA stack):

```bash
docker compose \
  -f docker/docker-compose.yml \
  -f docker/docker-compose.dev.yml \
  up --build db marreq-dev
```

This override adds `marreq-dev` (bind-mounted checkout + Cargo caches). The app is exposed at **http://localhost:8000**. To work on the SPA locally, run `npm run dev` in `frontend/` against a Rocket instance (e.g. `marreq-dev` or `cargo run -p marreq-server`) with CORS configured; see [doc/API.md](../doc/API.md).

For local development outside Docker, run either binary directly:

```bash
cargo run -p marreq-server   # standard server
cargo run -p marreq-cloud    # cloud variant
```

View logs:

```bash
docker compose -f docker/docker-compose.yml logs -f
```

Stop the stack:

```bash
docker compose -f docker/docker-compose.yml down
```

## Build images directly

Backend (`marreq-server`, the default):

```bash
docker build -f docker/Dockerfile -t marreq-backend:local .
```

To build the `marreq-cloud` binary instead:

```bash
docker build -f docker/Dockerfile --build-arg MARREQ_BIN=marreq-cloud -t marreq-cloud:local .
```

The `MARREQ_BIN` build-arg selects which workspace crate to compile. Valid values:

| `MARREQ_BIN` | Binary compiled | `Rocket.toml` source |
|---|---|---|
| `marreq-server` (default) | `target/release/marreq-server` | `marreq-server/Rocket.toml` |
| `marreq-cloud` | `target/release/marreq-cloud` | `marreq-cloud/Rocket.toml` |

Diesel migrations are always sourced from `marreq-core/migrations/` and placed at `/app/migrations` in the container.

Frontend (from repo root):

```bash
docker build -f docker/frontend/Dockerfile -t marreq-frontend:local ..
```

## CI Compose Overrides

When you need CI-like behavior locally, combine both files:

```bash
docker compose \
  -f docker/docker-compose.yml \
  -f docker/docker-compose.ci.yml \
  up -d db
```

## Script Compatibility

The DB helper scripts in `marreq-core/scripts/` already use `docker/docker-compose.yml` internally, so existing commands like `./marreq-core/scripts/db_setup.sh` keep working.

## Troubleshooting

### Port 5432 already in use

Compose maps the DB to host **5433**, not 5432, so it should not collide with system PostgreSQL. If you still see bind errors, another service may be using **5433** — change the mapping in `docker-compose.yml` (e.g. `5434:5432`).

### Port 11434 already in use

Compose maps Ollama to host **11435**, not 11434, so it should not collide with a host-installed Ollama. To call the **container** Ollama from your machine (e.g. `curl`), use `http://localhost:11435`. If **11435** is taken, change the mapping (e.g. `11436:11434`).

### `InsecureSecretKey` / backend exits after migrations

The container runs a **release** binary; Rocket needs **`ROCKET_SECRET_KEY`** (256-bit, `openssl rand -base64 32`). `docker-compose.yml` injects a **development default** when the variable is unset. If you removed it or use a custom compose file, set `ROCKET_SECRET_KEY` in `.env`.

### Orphan containers

If Compose warns about orphan containers (old service names), run:

```bash
docker compose -f docker/docker-compose.yml up -d --remove-orphans
```

### Database Connection Issues

```bash
# Check if database container is running
docker compose -f docker/docker-compose.yml ps db

# Check database connectivity
docker compose -f docker/docker-compose.yml exec -T db psql -U rust -d marreq -c "SELECT 1;"

# Restart database container
docker compose -f docker/docker-compose.yml restart db
```

### Database Reset

```bash
# Complete database reset
docker compose -f docker/docker-compose.yml exec -T db psql -U rust -d postgres -c "DROP DATABASE IF EXISTS marreq;"
./marreq-core/scripts/db_setup.sh --seed
```

### Verification Commands

```bash
# Verify database setup
docker compose -f docker/docker-compose.yml exec -T db psql -U rust -d marreq -c "\dt"

# Check user creation
docker compose -f docker/docker-compose.yml exec -T db psql -U rust -d marreq -c "SELECT username, name, is_admin FROM users;"

# Verify sample data
docker compose -f docker/docker-compose.yml exec -T db psql -U rust -d marreq -c "SELECT COUNT(*) as requirements FROM requirements;"
```

### SPA cannot reach API

Ensure you open the app on the **frontend** port (**8080**), not only the backend. The browser must call `/api/...` on the same host/port as the SPA so cookies are first-party.

### nginx `502` / `connect() failed (111: Connection refused)` to upstream

Usually means the **frontend** container started before Rocket was listening (migrations/seed) or nginx had a **stale IP** for `backend` after a recreate. The stack uses a **backend `healthcheck`** and **`depends_on: condition: service_healthy`** so nginx starts only after `GET /api/auth/csrf` succeeds on the backend; nginx is configured with **Docker DNS resolver** + variable `proxy_pass` so `backend` is re-resolved. Rebuild the backend image (it includes `curl` for the healthcheck) and recreate: `docker compose up -d --build backend frontend`.
