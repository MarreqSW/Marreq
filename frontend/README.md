# Marreq frontend (React + Vite)

SPA built with **React 19**, **TypeScript**, **Tailwind CSS**, and **React Flow** for the traceability graph. API calls use same-origin **`/api`** (Vite dev proxy → `http://127.0.0.1:8000`).

## Scripts

- `npm install` — dependencies
- `npm run dev` — Vite dev server (default `http://localhost:5173`), proxies `/api` → `http://127.0.0.1:8000`
- `npm run build` — typecheck + production build to `dist/`
- `npm run preview` — serve `dist/` locally (default `http://localhost:4173`); **`/api` is proxied to `http://127.0.0.1:8000` like dev** — start the backend first

## Routes (MVP)

- `/login` — JSON login (`POST /api/auth/login` with CSRF)
- `/register` — Cloud-mode self-service registration (`POST /api/auth/register`)
- `/verify-email` — consumes Cloud-mode email verification links
- `/forgot-password` and `/reset-password` — Cloud-mode password reset
- `/` — redirects to `/p/{selectedOrFirstProjectId}/requirements`
- `/p/:projectId/requirements` — requirements table (row opens editor)
- `/p/:projectId/requirements/:requirementId/edit` — edit requirement (Stitch / Axiom-style layout)
- `/p/:projectId/traceability` — matrix graph (requirement ↔ verification)

`projectId` is the **numeric** project id (same as `selected_project_id` from `GET /api/dashboard`), not the slug.

## API mapping (vs generic `/api/v1/…` guides)

| Guide / placeholder | Marreq endpoint |
|---------------------|-----------------|
| `GET /api/v1/projects/{id}/requirements` | `GET /api/projects/{project_id}/requirements` |
| Matrix / trace links | `GET /api/projects/{project_id}/matrix` |
| Requirement status labels | `GET /api/status` (join `status_id` on each requirement) |
| Verifications (for coverage denominator) | `GET /api/verifications` (filter by `project_id` client-side) |
| Session + CSRF | `GET /api/dashboard`, `GET /api/auth/csrf`; mutating calls need `X-CSRF-Token` |
| Deployment capabilities | `GET /api/meta/deployment` |
| Cloud auth | `POST /api/auth/register`, `GET /api/auth/verify-email`, `POST /api/auth/forgot-password`, `POST /api/auth/reset-password` |

See [doc/API.md](../doc/API.md) for the full HTTP contract.

## Docker

Built from `docker/frontend/Dockerfile` (build context: repository root). Nginx serves `dist/` and proxies `/api/` to the `backend` service. Run `npm install` + `npm run build` inside the image as today; no Dockerfile change required for this stack.

## Legacy static (optional)

`vite.config.ts` keeps a `@static` alias to `frontend/static` (legacy JS/CSS from the pre-React SPA) for gradual migration. The React shell does not import those assets by default.
