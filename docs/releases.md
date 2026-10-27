# Releases and dual artifact versioning

Marreq ships independently versioned artifacts:

| Artifact | Version source of truth | Compatibility declaration |
| --- | --- | --- |
| Backend (`marreq-server` / `marreq-cloud`) | `marreq-core/Cargo.toml` `version` | `marreq-core/frontend_compatibility.json` |
| Frontend (SPA) | `frontend/package.json` `version` | `frontend/compatibility.json` |
| MCP server | `mcp-server/package.json` `version` | — |

Both start at **`0.1.0`**. Release builds override the manifest version with the tag version (see [Runtime version](#runtime-version)), so the manifest value is only what local/dev builds report. Server and cloud share one API surface and therefore **one** frontend compatibility matrix.

## Runtime check

At runtime the SPA calls `GET /api/meta/build` and compares:

1. UI version against the backend’s `frontend_compatibility` `{ min_version, max_version }` (inclusive).
2. Backend version against the UI’s `requires_backend_min` / `requires_backend_max` (inclusive).

Mismatch shows a non-blocking amber banner. Help shows both versions (with short git SHAs), deployment mode, and compatible yes/no. The project sidebar footer and the login page show `UI {version} · API {version}`.

Ranges are plain semver strings (`major.minor.patch`), not npm-style range syntax.

## Compatibility matrix

| Backend | Compatible frontend |
| --- | --- |
| `0.1.0` – `0.2.99` | `0.1.0` – `0.2.99` |

| Frontend | Compatible backend |
| --- | --- |
| `0.1.0` – `0.2.99` | `0.1.0` – `0.2.99` |

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
| `marreq-mcp-vX.Y.Z` | MCP server image (version `X.Y.Z`) |

Pushing a matching tag runs `.github/workflows/release-artifacts.yml`, which builds and pushes the corresponding Docker image to GHCR (`ghcr.io/<owner>/marreq-server`, `marreq-frontend`, or `marreq-mcp-server`) tagged with the semver and `latest`.

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

# MCP server
docker build -f mcp-server/Dockerfile \
  -t marreq-mcp-server:0.1.0 .
```

Images carry OCI label `org.opencontainers.image.version`.

## Runtime version

`MARREQ_VERSION` (the tag with its `marreq-*-v` prefix stripped, e.g. `0.1.3`) and `MARREQ_GIT_SHA` are baked in at build time:

- Backend: `option_env!` in `marreq-core/src/build_info.rs`; reported as `backend_version` / `backend_git_sha` by `GET /api/meta/build`.
- Frontend: Vite `define` (`__FRONTEND_VERSION__`, `__FRONTEND_GIT_SHA__`) in `frontend/vite.config.ts`.

When `MARREQ_VERSION` is unset or empty, each artifact falls back to its manifest version. A leading `v` is stripped so the value stays plain `X.Y.Z` for the compatibility check. Local images: `make docker-server MARREQ_VERSION=0.1.3`.

## Exact artifact API compatibility (#423)

The Rust CLI checks each consumer's own retained contract, never current-main
substitutes. A bundle contains only its selected operations and reachable schemas,
component/version/source SHA and compatibility policy. Backend bundles contain
verified producer operations for one variant; unknown unconsumed routes are not
invented. Consumer export fails on any required unknown/unverified dependency.

Export from the artifact's clean source commit:

```sh
cargo run --locked -p marreq-contracts -- bundle --component backend --variant server --version 0.2.4 --output backend.json
cargo run --locked -p marreq-contracts -- bundle --component frontend --version 0.2.1 --output frontend.json
cargo run --locked -p marreq-contracts -- bundle --component mcp --version 0.3.0 --output mcp.json
```

Each command must run on **that component version's actual source**, not all on
main. Retain the output beside its release artifact. `marreq.contract.v2` requires
consumer-specific evidence; the old whole-backend bootstrap bundle is not
retrospectively certified. Never fabricate contracts for already published images.

A `marreq.deployment-contract.v2` plan names `variant` (`server`/`cloud`) and
`backend`, `frontend`, `mcp`. Each descriptor contains exactly `file`, `sha256`,
`version`, `source_sha`. Files must remain inside the plan directory. SHA-256 pins
**contract bytes, not Docker image provenance**; a trusted orchestrator must bind
the bundle/source to the actual image digest. This prerequisite does not implement
that future #423 release/provenance/CD architecture.

```json
{
  "format": "marreq.deployment-contract.v2",
  "variant": "server",
  "backend": {"file": "backend.json", "sha256": "<64 hex>", "version": "0.2.4", "source_sha": "<40 hex>"},
  "frontend": {"file": "frontend.json", "sha256": "<64 hex>", "version": "0.2.1", "source_sha": "<40 hex>"},
  "mcp": {"file": "mcp.json", "sha256": "<64 hex>", "version": "0.3.0", "source_sha": "<40 hex>"}
}
```

```sh
cargo run --locked -p marreq-contracts -- verify --manifest deployment.json
# Equivalent thin wrapper:
scripts/check-api-compatibility.sh --manifest deployment.json
```

For this backend-only update, retain the actual frontend 0.2.1 and MCP 0.3.0
bundles. The candidate 0.2.4 must satisfy their request/response contracts and each
consumer's backend range. Equal component versions are not required. The backend's
existing frontend acceptance range is also enforced. An unrelated backend route
outside both consumer bundles can change without failing their compatibility.

Optional request fields and additional response fields are compatible where the
old contract accepts them. Removed guaranteed response properties, new required
request properties, narrowed request enums, widened closed response enums,
incompatible types/authentication, or missing required operations fail. Per-operation
Bearer scopes and stable CSRF policy IDs are checked separately; unused global
security schemes and exact global status/header set equality do not couple APIs.
The pinned standards engine decides ordinary OpenAPI changes, not homegrown rules.
Missing evidence, altered hashes, mismatched source/version, invalid policy,
unsupported semantics and uncertain required operations exit nonzero with JSON
errors. Version ranges supplement executable contracts and never prove them alone.
During rolling upgrades, check every old/new combination that may coexist.

Current component-tag publishing is retained. A publication job checks out an
immutable plan revision from `MARREQ_RELEASE_CONTRACTS_SHA`, exports the exact tagged
candidate, and verifies its version, source and bundle hash against the plan before
any image publisher runs. The release gate uses the **actual artifact combination**;
it does not require the latest checkout's unchanged consumers. The PR gate instead
checks all calls in current source. No gate requires coverage of unconsumed routes.
No global release migration or production deployment is performed here.

The PR remains draft until required consumer schema gaps and security failures are
resolved; a document-only check is not a merge/release-readiness assertion.
