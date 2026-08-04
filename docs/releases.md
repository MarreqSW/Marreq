# Releases and dual artifact versioning

Marreq ships two independently versioned artifacts:

| Artifact | Version source of truth | Compatibility declaration |
| --- | --- | --- |
| Backend (`marreq-server` / `marreq-cloud`) | `marreq-core/Cargo.toml` `version` | `marreq-core/frontend_compatibility.json` |
| Frontend (SPA) | `frontend/package.json` `version` | `frontend/compatibility.json` |

Both start at **`0.1.0`**. Server and cloud share one API surface and therefore **one** frontend compatibility matrix.

## Runtime check

At runtime the SPA calls `GET /api/meta/build` and compares:

1. UI version against the backend’s `frontend_compatibility` `{ min_version, max_version }` (inclusive).
2. Backend version against the UI’s `requires_backend_min` / `requires_backend_max` (inclusive).

Mismatch shows a non-blocking amber banner. Help shows both versions, deployment mode, and compatible yes/no. Login shows `UI {version}` only.

Ranges are plain semver strings (`major.minor.patch`), not npm-style range syntax.

## Compatibility matrix

| Backend | Compatible frontend |
| --- | --- |
| `0.1.0` | `0.1.0` – `0.1.99` |

| Frontend | Compatible backend |
| --- | --- |
| `0.1.0` | `0.1.0` – `0.1.99` |

Widen or tighten the JSON files when introducing a breaking API or UI contract change. Bump the corresponding package version in the same change.

## Bump rules

1. **Patch / minor (compatible):** bump only the artifact that changed; leave the other at its current version if the matrix still covers it.
2. **Breaking API:** bump backend version, update `frontend_compatibility.json` for the new UI range, and bump frontend once it requires the new API (`requires_backend_*`).
3. **Breaking UI expectations:** bump frontend and update `requires_backend_*`; update backend `frontend_compatibility` if older UIs must be rejected.
4. Do not invent a second matrix for cloud; keep server and cloud in lockstep for this contract.

## Git tags

| Tag pattern | Artifact |
| --- | --- |
| `marreq-server-vX.Y.Z` | Backend image / binary (version `X.Y.Z`) |
| `marreq-frontend-vX.Y.Z` | Frontend image (version `X.Y.Z`) |

Pushing a matching tag runs `.github/workflows/release-artifacts.yml`, which builds and pushes the corresponding Docker image to GHCR (`ghcr.io/<owner>/marreq-server` or `marreq-frontend`) tagged with the semver and `latest`.

## Manual Docker builds

```bash
# Backend (from repo root)
docker build -f docker/Dockerfile \
  --build-arg MARREQ_BIN=marreq-server \
  --build-arg MARREQ_VERSION=0.1.0 \
  --build-arg MARREQ_GIT_SHA="$(git rev-parse HEAD)" \
  -t marreq-server:0.1.0 .

# Frontend
docker build -f docker/frontend/Dockerfile \
  --build-arg MARREQ_VERSION=0.1.0 \
  --build-arg MARREQ_GIT_SHA="$(git rev-parse HEAD)" \
  -t marreq-frontend:0.1.0 .
```

Images carry OCI label `org.opencontainers.image.version`. Backend builds inject `MARREQ_GIT_SHA` into the binary (`option_env!`); frontend builds bake version and SHA via Vite `define`.
