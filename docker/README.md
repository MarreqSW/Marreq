# Docker Layout and Usage

All container-related files live in this directory.

Copy the project env template before first use:

```bash
cp .env.example .env
```

For the end-to-end setup flows for **`marreq-server`** and **`marreq-cloud`** (Docker and non-Docker), start with [../docs/developer/setup.md](../docs/developer/setup.md).

## Files

- `docker-compose.yml`: Primary local stack — **`db`**, **`ollama`**, **`marreq-server`** (Rocket API; default profile), **`marreq-cloud`** (Rocket API; `cloud` profile), **`frontend`** (self-hosted nginx + SPA), **`frontend-cloud`** (cloud nginx + SPA; `cloud` profile), **`adminer`**
- `docker-compose.prod.yml`: Production hardening override for HTTPS deployments behind an external TLS reverse proxy
- `docker-compose.dev.yml`: Developer override for running Marreq via `cargo run` inside Docker (`marreq-dev` on host port **8000**)
- `docker-compose.ci.yml`: CI-specific compose overrides
- `Dockerfile`: Marreq image (Rust binary; build context: repository root). Accepts `MARREQ_BIN` build-arg (`marreq-server` by default; `marreq-cloud` for the cloud variant).
- `frontend/Dockerfile`: **Frontend** image (multi-stage: `npm run build` + nginx)
- `frontend/nginx.conf.template`: nginx template used by both frontend containers; serves the SPA and proxies `/api/` to either `marreq-server:8000` or `marreq-cloud:8001`
- `Dockerfile.dockerignore`: Build context exclusions for `Dockerfile`
- `docker-entrypoint.sh`: Backend container startup (wait for DB + migrations + start app)

## Split stack (default compose)

The default `docker-compose.yml` uses a **split SPA stack**: nginx serves the Vite build and proxies `/api/` to the selected Rocket backend so the browser stays on a single origin. Host-published services bind to loopback by default so they are not exposed on public network interfaces.

| Service   | Role |
|-----------|------|
| `db` | PostgreSQL published on host **`127.0.0.1:5433`** → container `5432` (avoids conflict with a local Postgres on **5432**). From the host, use `DATABASE_URL=postgres://rust:rust@127.0.0.1:5433/marreq` for `diesel`/scripts. |
| `ollama` | Published on host **`127.0.0.1:11435`** → container `11434` (avoids conflict with a local Ollama on **11434**). The Marreq container still uses `http://ollama:11434` on the Docker network. |
| `marreq-server` | Self-hosted Rocket binary on **`127.0.0.1:8000`**: **`/api`**, plus HTML + **`/static`** when **`MARREQ_DOCKER_SSR_PROXY=1`**. **`GET /`** on :8000 is the classic dashboard; use **:8080/** for the SPA. **`ROCKET_SECRET_KEY`**: compose default if missing. |
| `marreq-cloud` | Hosted (SaaS) Rocket binary on **`127.0.0.1:8001`**. Started only by the **`cloud`** compose profile. Reads cloud-only env (`MARREQ_SITE_ADMIN_EMAIL`, `MARREQ_SITE_ADMIN_BOOTSTRAP_PASSWORD`, `MARREQ_PUBLIC_BASE_URL`, `SMTP_*`) from `../.env`. Compose defaults `MARREQ_PUBLIC_BASE_URL` to **http://localhost:8082**. |
| `frontend` | Nginx: SPA on **http://127.0.0.1:8080** (configurable with `MARREQ_FRONTEND_PORT`) with `/api/` proxied to **`marreq-server:8000`**. |
| `frontend-cloud` | Nginx: same SPA on host **http://127.0.0.1:8082** (configurable with `MARREQ_CLOUD_FRONTEND_PORT`) with `/api/` proxied to **`marreq-cloud:8001`**. Started only by the **`cloud`** compose profile. |
| `adminer` | Database UI on host **http://127.0.0.1:8081** (avoids clashing with frontend **8080**). |

Use the UI at **http://localhost:8080** for self-hosted mode, or **http://localhost:8082** for cloud mode, so session cookies stay on the same origin as `/api`.

## Common Commands (from repo root)

Start only the database:

```bash
docker compose -f docker/docker-compose.yml up -d db
```

Start the full self-hosted stack (db, ollama, marreq-server, frontend, adminer):

```bash
docker compose -f docker/docker-compose.yml up -d
```

Start the cloud variant with its own frontend:

```bash
docker compose -f docker/docker-compose.yml --profile cloud up -d marreq-cloud frontend-cloud
```

The cloud service expects the following entries in `../.env`:
`MARREQ_SITE_ADMIN_EMAIL`, `MARREQ_SITE_ADMIN_BOOTSTRAP_PASSWORD`,
`MARREQ_PUBLIC_BASE_URL`, and the `SMTP_*` block (see
[../docs/developer/workspace-layout.md](../docs/developer/workspace-layout.md#cloud-mode-environment-variables)).

The Docker Compose files load the project `../.env` for shared app settings.
Docker-specific connection values such as the in-container `DATABASE_URL` and
`OLLAMA_URL` stay in the Compose files so they do not duplicate host-local
values in `.env`.

## Production HTTPS deployment

Marreq does not need to terminate TLS inside Rocket. In production, run the normal split stack on loopback and terminate TLS in a host-level reverse proxy such as Caddy, nginx, or Traefik. The reverse proxy owns public ports **80/443** and forwards the HTTPS origin to the frontend on **127.0.0.1:8080**. The frontend nginx then serves the SPA and proxies `/api/` to Rocket over the internal Docker network.

### 1. Configure DNS and environment

Point the public hostname (for example `marreq.example.com`) at the server, then set production values in the repository-root `.env`:

```dotenv
ROCKET_SECRET_KEY=<output of: openssl rand -base64 32>
MARREQ_SECURE_SESSION_COOKIE=1
MARREQ_PUBLIC_BASE_URL=https://marreq.example.com
CSRF_ALLOWED_ORIGINS=https://marreq.example.com
```

`docker-compose.prod.yml` requires `ROCKET_SECRET_KEY`, `MARREQ_PUBLIC_BASE_URL`, and `CSRF_ALLOWED_ORIGINS`; Compose fails during configuration if any of them are missing. It also forces secure session cookies for the backend.

### 2. Start the production stack

Validate the merged Compose configuration first:

```bash
docker compose \
  -f docker/docker-compose.yml \
  -f docker/docker-compose.prod.yml \
  config >/dev/null
```

Then build and start the self-hosted production services:

```bash
docker compose \
  -f docker/docker-compose.yml \
  -f docker/docker-compose.prod.yml \
  up -d --build db ollama marreq-server frontend
```

Adminer is opt-in with the production override. If it is needed temporarily, start it with the `admin` profile and keep access local or through an SSH tunnel:

```bash
docker compose \
  -f docker/docker-compose.yml \
  -f docker/docker-compose.prod.yml \
  --profile admin up -d adminer
```

### 3. Terminate TLS in the host reverse proxy

For example, a minimal Caddy configuration is:

```caddyfile
marreq.example.com {
    reverse_proxy 127.0.0.1:8080
}
```

With DNS pointing to the server and ports 80/443 reachable, Caddy can obtain and renew the public certificate automatically. Equivalent nginx or Traefik configurations are also valid.

The frontend nginx preserves a trusted incoming `X-Forwarded-Proto: https` value when proxying `/api/`, so the original public scheme survives the proxy chain. Direct local access falls back to nginx's own request scheme.

### 4. Firewall and exposure

A typical production host only needs public inbound access to:

- **80/tcp** — HTTP, normally redirected to HTTPS by the TLS proxy
- **443/tcp** — HTTPS
- **22/tcp** — SSH, if the host is administered over SSH

The default Compose stack binds PostgreSQL (`5433`), Ollama (`11435`), Rocket (`8000`/`8001`), the SPA frontend (`8080`/`8082`), and Adminer (`8081`) to `127.0.0.1`, so these ports are not directly exposed on public host interfaces.

### 5. Verify the deployment

After the proxy is configured, verify the public origin rather than the internal `:8080` endpoint:

```bash
curl -I https://marreq.example.com/
curl -I https://marreq.example.com/api/auth/csrf
```

The browser should receive Marreq over HTTPS and authenticated sessions should use the secure `__Host-session` cookie.

Start a Docker-only developer loop (Rocket **with** classic HTML UI on port 8000 — not the split SPA stack):

```bash
docker compose \
  -f docker/docker-compose.yml \
  -f docker/docker-compose.dev.yml \
  up --build db marreq-dev
```

This override adds `marreq-dev` (bind-mounted checkout + Cargo caches). The app is exposed at **http://localhost:8000**. To work on the SPA locally, run `npm run dev` in `frontend/` against a Rocket instance (e.g. `marreq-dev`, `cargo run -p marreq-server`, or `cargo run -p marreq-cloud`) with CORS configured; see [../docs/developer/http-api-contract.md](../docs/developer/http-api-contract.md).

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

Build the self-hosted server image (`marreq-server`, the default):

```bash
docker build -f docker/Dockerfile -t marreq-server:local .
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

Compose maps the DB to host **5433**, not 5432, so it should not collide with system PostgreSQL. If you still see bind errors, another local service may be using **5433** — change the loopback mapping in `docker-compose.yml` (e.g. `127.0.0.1:5434:5432`).

### Port 11434 already in use

Compose maps Ollama to host **11435**, not 11434, so it should not collide with a host-installed Ollama. To call the **container** Ollama from your machine (e.g. `curl`), use `http://localhost:11435`. If **11435** is taken, change the loopback mapping (e.g. `127.0.0.1:11436:11434`).

### `InsecureSecretKey` / Rocket exits after migrations

The container runs a **release** binary; Rocket needs **`ROCKET_SECRET_KEY`** (256-bit, `openssl rand -base64 32`). `docker-compose.yml` injects a **development default** when the variable is unset. The production override rejects a missing key, so set `ROCKET_SECRET_KEY` in `.env` before using `docker-compose.prod.yml`.

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

Ensure you open the app on the **frontend** port (**8080**), not only the API port. The browser must call `/api/...` on the same host/port as the SPA so cookies are first-party.

### nginx `502` / `connect() failed (111: Connection refused)` to upstream

Usually means a frontend container started before its backend was listening (migrations/seed) or nginx had a **stale IP** for `marreq-server` / `marreq-cloud` after a recreate. The stack uses backend `healthcheck`s and `depends_on: condition: service_healthy` so nginx starts only after `GET /api/auth/csrf` succeeds on the target backend; nginx is configured with **Docker DNS resolver** + variable `proxy_pass` so upstream names are re-resolved. Rebuild the image (it includes `curl` for the healthcheck) and recreate, for example: `docker compose up -d --build marreq-server frontend` or `docker compose --profile cloud up -d --build marreq-cloud frontend-cloud`.
