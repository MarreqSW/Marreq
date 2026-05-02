# Setup guide

This is the canonical setup guide for local Marreq environments. It explains how to run **`marreq-server`** and **`marreq-cloud`** both **with Docker** and **without Docker**, and links out to the more specialized references when you need deeper detail.

## Choose your deployment mode

| Binary | Best for | User onboarding | Typical local entry point |
|---|---|---|---|
| `marreq-server` | Self-hosted deployments | Admin-created users | Docker SPA on `http://localhost:8080` or direct Rocket on `http://127.0.0.1:8000` |
| `marreq-cloud` | Hosted / SaaS-style deployments | Site-admin bootstrap + self-registration | Docker API on `http://127.0.0.1:8001` or direct Rocket on `http://127.0.0.1:8000` |

For architecture and deployment-mode differences, see [workspace-layout.md](workspace-layout.md).

## Shared prerequisites

- **Rust** toolchain (`cargo`)
- **Node.js + npm** for the SPA (`frontend/`)
- **PostgreSQL** locally, or **Docker Compose** for the bundled database
- **Diesel CLI** for schema setup:
  ```bash
  cargo install diesel_cli --no-default-features --features postgres
  ```
- **Optional Ollama** if you want embeddings / semantic search
- **SMTP** if you want `marreq-cloud` email verification and password-reset flows

## Shared preparation

1. Copy the environment template:
   ```bash
   cp .env.example .env
   ```
2. Choose your database target:
   - **Docker DB**: keep the default `DATABASE_URL=postgres://rust:rust@127.0.0.1:5433/marreq`
   - **Local PostgreSQL**: update `DATABASE_URL` in `.env`
3. Initialize the database schema:
   ```bash
   ./marreq-core/scripts/db_setup.sh
   ```
4. Optional: load demo projects, users, and traceability data:
   ```bash
   ./marreq-core/scripts/db_setup.sh --seed
   ```

### What the shared setup gives you

- `db_setup.sh` creates the `marreq` database if needed and runs Diesel migrations.
- On a fresh empty database, the baseline migration creates a default admin user: **`alice` / `ChangeMe123!`**.
- `--seed` adds the rest of the demo users, projects, requirements, verifications, and sample matrix data.

For database internals, reset flows, and manual migration commands, see [database-setup.md](database-setup.md).

## `marreq-server` with Docker

Use this when you want the standard self-hosted stack with the bundled frontend container.

1. Prepare `.env` and initialize the database:
   ```bash
   cp .env.example .env
   ./marreq-core/scripts/db_setup.sh --seed
   ```
2. Start the stack:
   ```bash
   docker compose -f docker/docker-compose.yml up -d
   ```
3. Open the SPA at **`http://localhost:8080`**.
4. Log in with **`alice` / `ChangeMe123!`** or your own admin account.

Notes:

- The Docker frontend proxies `/api` to `marreq-server`.
- The backend is also reachable directly on **`http://127.0.0.1:8000/api`**.
- If you do not want demo data, omit `--seed`; the baseline admin `alice` is still present on a brand-new empty database.

For container-level details, service names, health checks, and direct image builds, see [../../docker/README.md](../../docker/README.md).

## `marreq-server` without Docker

Use this when you want to run Rocket directly from your checkout.

1. Point `.env` at your local PostgreSQL instance if you are not using the Docker DB.
2. Initialize the database:
   ```bash
   ./marreq-core/scripts/db_setup.sh --seed
   ```
3. Start the server binary:
   ```bash
   cargo run -p marreq-server
   ```
4. Either:
   - open Rocket directly at **`http://127.0.0.1:8000`**, or
   - run the SPA dev server:
     ```bash
     cd frontend
     npm install
     npm run dev
     ```
     then use **`http://localhost:5173`**.

Notes:

- The Vite dev proxy already targets **`http://127.0.0.1:8000`**, so it works with the default `marreq-server` port.
- If you skip `--seed`, a brand-new empty database still includes the default admin `alice`.

## `marreq-cloud` with Docker

Use this when you want the hosted/SaaS deployment binary in containers.

1. Prepare `.env`:
   ```bash
   cp .env.example .env
   ```
2. Set the cloud-only variables in `.env`:
   - `MARREQ_SITE_ADMIN_EMAIL`
   - `MARREQ_SITE_ADMIN_BOOTSTRAP_PASSWORD`
   - `MARREQ_PUBLIC_BASE_URL`
   - `SMTP_HOST`, `SMTP_PORT`, `SMTP_USERNAME`, `SMTP_PASSWORD`, `SMTP_FROM_ADDRESS` (required for email verification / reset links to actually send)
3. Initialize the database:
   ```bash
   ./marreq-core/scripts/db_setup.sh
   ```
4. Start the cloud binary:
   ```bash
   docker compose -f docker/docker-compose.yml --profile cloud up -d marreq-cloud
   ```
5. Use the API at **`http://127.0.0.1:8001/api`**.

Notes:

- `marreq-cloud` bootstraps or promotes the configured site admin at startup.
- Set `MARREQ_PUBLIC_BASE_URL` to the actual browser origin that should appear in cloud email links.
- The bundled Docker `frontend` service is wired to **`marreq-server`**, not `marreq-cloud`. For cloud-mode SPA testing, run the SPA outside Docker against a cloud backend, or provide your own reverse proxy in front of `marreq-cloud`.

## `marreq-cloud` without Docker

Use this when you want the simplest local cloud-mode setup with the React SPA.

1. Prepare `.env` and set the same cloud-only variables as above.
2. Initialize the database:
   ```bash
   ./marreq-core/scripts/db_setup.sh
   ```
3. Start the cloud binary:
   ```bash
   cargo run -p marreq-cloud
   ```
4. Optional but recommended: run the SPA locally:
   ```bash
   cd frontend
   npm install
   npm run dev
   ```
5. Open:
   - **`http://localhost:5173`** for the SPA, or
   - **`http://127.0.0.1:8000/api`** for direct API access.

Notes:

- The default Vite proxy target is **`http://127.0.0.1:8000`**, so local `marreq-cloud` works with the SPA without extra proxy changes as long as you keep the default Rocket port.
- If you need to run `marreq-server` and `marreq-cloud` side by side outside Docker, start one of them on another port, for example:
  ```bash
  ROCKET_PORT=8001 cargo run -p marreq-cloud
  ```
  In that case, update your SPA proxy or other client configuration to match.
- Without SMTP, the cloud site admin can still be bootstrapped, but end-user email verification and password-reset delivery will not work.

## Related docs

- [Workspace layout & deployment modes](workspace-layout.md)
- [Database setup](database-setup.md)
- [HTTP API contract](http-api-contract.md)
- [Frontend README](../../frontend/README.md)
- [Docker / Compose reference](../../docker/README.md)
