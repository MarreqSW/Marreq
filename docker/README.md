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
- `Dockerfile`: **Backend** image (Rust binary; build context: repository root)
- `frontend/Dockerfile`: **Frontend** image (multi-stage: `npm run build` + nginx)
- `frontend/nginx.conf`: SPA `try_files` + `/api/` reverse proxy to `backend:8000`
- `Dockerfile.dockerignore`: Build context exclusions for `Dockerfile`
- `docker-entrypoint.sh`: Backend container startup (wait for DB + migrations + start app)

## Split stack (default compose)

The default `docker-compose.yml` runs the app in **API-only** mode plus a separate **frontend** container:

| Service   | Role |
|-----------|------|
| `backend` | Rocket on `:8000` **inside** the compose network (`expose`, not published to host by default). `MARREQ_UI_MODE=api_only`, `MARREQ_SERVE_STATIC=0`. |
| `frontend` | Nginx serves the Vite-built SPA on host **http://localhost:8080** and proxies **`/api/`** → `http://backend:8000/api/`. |
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

This override adds `marreq-dev` (bind-mounted checkout + Cargo caches). The app is exposed at **http://localhost:8000**. To work on the SPA locally, run `npm run dev` in `frontend/` against a Rocket instance (e.g. `marreq-dev` or `cargo run`) with CORS configured; see [doc/API.md](../doc/API.md).

View logs:

```bash
docker compose -f docker/docker-compose.yml logs -f
```

Stop the stack:

```bash
docker compose -f docker/docker-compose.yml down
```

## Build images directly

Backend:

```bash
docker build -f docker/Dockerfile -t marreq-backend:local ..
```

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

The DB helper scripts in `scripts/` already use `docker/docker-compose.yml` internally, so existing commands like `./scripts/db_setup.sh` keep working.

## Troubleshooting

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
./scripts/db_setup.sh --seed
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
