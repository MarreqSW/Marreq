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

## API compatibility gate and retained artifact contracts (#423 P0)

The P0 bootstrap is **not release-ready**: 206 HTTP operations still lack audited
wire contracts. `API contracts` CI validates the document, generated artifacts,
source-review evidence, both compiled route inventories, negative historical
fixtures and representative real producer/consumer exchanges. Release publication
also requires `scripts/contracts/check.py --require-complete`, which currently
fails intentionally. This must remain a draft until the coverage gaps described in
[the HTTP contract audit](developer/http-api-contract.md#p0-contract-hardening-audit-and-current-coverage-423)
are closed.

### Compatibility policy

Component SemVer describes each artifact independently. Equal versions are never
required. A consumer declares `requires_backend_min` and optionally an inclusive
`requires_backend_max`; frontend and MCP metadata are validated independently.
The backend's existing frontend acceptance range is also checked. Stable
`X.Y.Z` versions are supported; prerelease and wildcard ranges fail closed.
Omitting an upper bound is appropriate only with executable retained-contract
checks; an upper bound is a temporary tested-support ceiling, not proof that
all versions inside it work. Existing runtime `CompatibilityBanner` warnings are
preserved for manually assembled deployments.

A producer must continue accepting every previously valid consumer request and
returning responses accepted by that released consumer. Additive optional request
fields and additive response fields are compatible when the consumer accepts
unknown properties. Removing an endpoint or guaranteed response property,
changing a field type/nullability, adding a required request field, restricting a
request enum, extending a response enum used by exhaustive consumers, changing
status/media/header behavior or changing authentication/scopes is potentially
breaking. Breaking changes require a new API-compatible deployment combination
and a component SemVer breaking release; while 0.x, treat minor changes with the
same explicit care rather than assuming all 0.x releases interoperate. Unknown
changed schema constraints/unions are rejected conservatively for manual review.

Previously released consumers retain their own contracts and policy. A newer
backend must satisfy those old contracts, not regenerated contracts from current
`main`. During rolling upgrades, check each backend/consumer combination that
can coexist, including old and new replicas; upgrade consumers first only when
the old backend also passes. No gate establishes database migration safety;
that remains a separate P0 workstream in #423.

### Export and retain an exact component contract

After committing audited contract inputs on the component's source commit:

```sh
python3 scripts/contracts/bundle.py --component backend --variant server \
  --version 0.2.4 --output backend-0.2.4.json
python3 scripts/contracts/bundle.py --component frontend \
  --version 0.2.1 --output frontend-0.2.1.json
python3 scripts/contracts/bundle.py --component mcp \
  --version 0.3.0 --output mcp-0.3.0.json
```

Run each command on **that artifact's own source commit**, not all on current
`main`. The exporter refuses dirty audited inputs and records the exact source
SHA. Retain the JSON next to the versioned image/release with its SHA-256. Images
also carry their build-time OpenAPI and compatibility metadata. The exporter can
record incomplete bootstrap evidence for review, but the gate cannot certify it.
Old releases without retained evidence require a real audit on their source
commit; missing evidence never falls back to current sources.

### Verify the resulting deployment, including unchanged artifacts

Prepare a `deployment.json` with exactly these top-level fields:

```json
{
  "format": "marreq.deployment-contract.v1",
  "variant": "server",
  "backend": {
    "file": "backend-0.2.4.json",
    "sha256": "<64 lowercase hex characters>",
    "version": "0.2.4",
    "source_sha": "<40 lowercase hex characters>"
  },
  "frontend": {
    "file": "frontend-0.2.1.json",
    "sha256": "<64 lowercase hex characters>",
    "version": "0.2.1",
    "source_sha": "<40 lowercase hex characters>"
  },
  "mcp": {
    "file": "mcp-0.3.0.json",
    "sha256": "<64 lowercase hex characters>",
    "version": "0.3.0",
    "source_sha": "<40 lowercase hex characters>"
  }
}
```

Place referenced bundles inside the plan directory. Verify offline:

```sh
scripts/check-api-compatibility.sh --manifest deployment.json
```

Exit 0 means all checked declarations/contracts are compatible. Exit 1 means
incompatibility, incomplete evidence, invalid metadata, wrong deployment variant,
a missing artifact, or a digest/source/version mismatch. Output is structured JSON
with component-specific diagnostics. The comparator checks request acceptance in
the consumer-to-producer direction and response acceptance in the reverse
direction. This command does not download artifacts, execute plans, send tokens,
publish images or deploy anything. SHA-256 pins content; trusted provenance for
retrieving artifacts remains the release/deployment orchestrator's responsibility.

### Current tag workflow and future global releases

`release-artifacts.yml` retains the existing component tag triggers. Every image
publisher depends on contract tests and the mandatory deployment gate. Once P0
coverage is complete, prepare a separate immutable revision in this repository
containing `deployment.json` and all referenced bundles; set the repository variable
`MARREQ_RELEASE_CONTRACTS_SHA` to that full commit SHA. This revision is prepared
**after** the candidate source commit exists, avoiding self-referential source
hashes in a manifest committed with its own candidate. The workflow checks out
that exact plan revision, exports the candidate contract from the tagged source,
and checks its digest, version and source SHA against the plan. Unchanged
frontend/MCP/backend bundles are never replaced with the tag checkout's versions.
The candidate contract is retained as a workflow artifact before image publishing.
No default production version or fabricated historical manifest is provided.

The future global-release/selective-deployment pipeline should retrieve trusted
retained bundles, construct every resulting deployment plan, and invoke this
same command plus the reusable `api-contracts.yml` workflow. Global version
registry/tag migration, image digest provenance, actual CD, database migrations,
rollback and VPS configuration remain in #423. Nothing here deploys to the VPS.
