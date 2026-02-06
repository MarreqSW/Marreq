# Rebuilding the Docker Image When Code Changes

This document explains how to rebuild the ReqMan Docker image after you change application code, templates, or static assets.

## When to Rebuild

Rebuild the image whenever you change:

- **Rust source** (`src/**/*.rs`)
- **Templates** (`templates/**`)
- **Static assets** (`src/html/static/**` — CSS, JS, etc.)
- **Migrations** (`migrations/**`)
- **Config** (`Rocket.toml`, `Cargo.toml`, `Cargo.lock`, `diesel.toml`)
- **Dockerfile** or **docker-entrypoint.sh**

## Quick Rebuild and Restart

From the project root (where `docker-compose.yml` lives):

```bash
docker compose build reqman && docker compose up -d reqman
```

- **`docker compose build reqman`** — Builds (or rebuilds) the `reqman` service image.
- **`docker compose up -d reqman`** — Starts (or restarts) the `reqman` container in the background.

If the app is already running, `up -d` will replace the container with one using the new image.

## Rebuild Without Using Cache

If something didn’t update as expected or you want a clean build:

```bash
docker compose build --no-cache reqman
docker compose up -d reqman
```

`--no-cache` forces Docker to run every build step again instead of reusing cached layers. Use this when:

- You suspect stale layers (e.g. old binaries or assets).
- You changed base image or system packages in the Dockerfile.
- You want to verify a clean build.

## Build-Only (No Restart)

To only rebuild the image and not touch running containers:

```bash
docker compose build reqman
```

To rebuild without cache:

```bash
docker compose build --no-cache reqman
```

Then start or restart when you’re ready:

```bash
docker compose up -d reqman
```

## How the Image Is Built

The ReqMan Dockerfile uses a multi-stage build:

1. **Builder stage** (Rust image):
   - Installs system deps and `diesel_cli`.
   - Copies `Cargo.toml` / `Cargo.lock` and builds dependencies (this layer is cached until those files change).
   - Copies `src/` and `migrations/`, then builds the release binary.
   - Static assets and templates are part of the source tree used in this stage.

2. **Runtime stage** (Debian slim):
   - Copies the compiled binary, migrations, `diesel`, `Rocket.toml`, static files, and templates from the builder.
   - Uses `docker-entrypoint.sh` to wait for the database and run migrations before starting the app.

So any change to Rust code, templates, or static files invalidates the “copy source and build” step and triggers a rebuild of the binary and the final image.

## One-Command Rebuild and Run

To always use a fresh build and then start the stack (e.g. after pulling or changing code):

```bash
docker compose up -d --build reqman
```

`--build` forces the image to be built (using cache when possible) before starting the container.

## Summary

| Goal                         | Command |
|-----------------------------|--------|
| Rebuild and restart app     | `docker compose build reqman && docker compose up -d reqman` |
| Rebuild with no cache       | `docker compose build --no-cache reqman` then `docker compose up -d reqman` |
| Build then run (with build) | `docker compose up -d --build reqman` |
| Only rebuild image          | `docker compose build reqman` |

For day-to-day development after code changes, **`docker compose build reqman && docker compose up -d reqman`** is usually enough.
