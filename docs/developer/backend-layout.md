# Backend layout (Rocket API)

The Marreq **Rust / Rocket application** lives at the **repository root** (`Cargo.toml`, `src/`, `migrations/`, `templates/` for legacy SSR). Docker continues to use the repo root as build context with `docker/Dockerfile`.

## API-only mode

Set `MARREQ_UI_MODE=api_only` to **omit** HTML routes (`/`, `/p/...`, `/user/...`) and use only `/api` (+ shared fairings/catchers). Intended for a split stack where **every** UI path is handled by the SPA.

## Hybrid Docker split (default compose)

`docker/docker-compose.yml` sets **`MARREQ_DOCKER_SSR_PROXY=1`** so the backend entrypoint does **not** force `api_only`. Nginx on **:8080** serves the Vite SPA for **`/`** only and **reverse-proxies** `/p/…`, `/static/…`, `/projects`, etc. to Rocket. Session cookies stay on one origin; project pages remain server-rendered until the SPA grows a client router.

## Static files

`MARREQ_SERVE_STATIC=0` disables Rocket’s `/static` `FileServer`. Default: static is **off** when `api_only`, **on** otherwise (classic `cargo run` UX).

## Related docs

- [doc/API.md](../../doc/API.md) — interchangeable-frontend contract
- [docker/README.md](../../docker/README.md) — compose services `backend` + `frontend`
