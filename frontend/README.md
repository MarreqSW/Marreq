# Marreq frontend (Vite SPA)

Bundled SPA that **reuses** legacy scripts and styles from `src/html/static/` via the Vite alias `@static` (see `vite.config.ts`).

## Scripts

- `npm install` — dependencies
- `npm run dev` — Vite dev server (default `http://localhost:5173`), proxies `/api` → `http://127.0.0.1:8000`
- `npm run build` — typecheck + production build to `dist/`
- `npm run preview` — serve `dist/` locally (default `http://localhost:4173`); **`/api` is proxied to `http://127.0.0.1:8000` like dev** — start the backend first

## Environment

- `VITE_API_BASE` — optional prefix for API calls (default empty: use same-origin `/api` when behind nginx, or rely on dev proxy).

## Docker

The image is built from `docker/frontend/Dockerfile` (context: repository root). Nginx serves `dist/` and proxies `/api/` to the `backend` service.

See [doc/API.md](../doc/API.md) for the HTTP contract (auth, CSRF, cookies).
