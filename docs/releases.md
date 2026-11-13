# Releases and dual artifact versioning

The database safety prerequisite of #423 uses [controlled migration jobs and coherent recovery bundles](developer/database-migrations.md). Both backend images share the same serialized Diesel runner; startup only validates history. Future release automation must stop progression on backup, restore rehearsal, migration, schema or real application smoke failure. Retained binaries need an explicitly tested expansion compatibility pin; database rollback is never automatic.

Marreq ships independently versioned artifacts:

| Artifact | Version source of truth | Compatibility declaration |
| --- | --- | --- |
| Server | `marreq-server/Cargo.toml` | `marreq-core/frontend_compatibility.json` |
| Cloud | `marreq-cloud/Cargo.toml` | `marreq-core/frontend_compatibility.json` |
| Frontend (SPA) | `frontend/package.json` `version` | `frontend/compatibility.json` |
| MCP server | `mcp-server/package.json` `version` | `mcp-server/compatibility.json` |

These manifests supply development defaults. Release builds use an explicit component version and source commit. Server and Cloud share the frontend compatibility range.

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

- Backend: compile-time values in each deployable crate, reported as `backend_version` / `backend_git_sha` by `GET /api/meta/build`. `deployment_mode` identifies Server or Cloud.
- Frontend: Vite embeds the values in the SPA and serves them at `/build-info.json`.
- MCP: `npm run build` generates immutable packaged metadata. The SDK advertises its version in STDIO and HTTP handshakes. HTTP exposes `/build-info` separately from `/healthz`.

When `MARREQ_VERSION` is unset or empty, each artifact falls back to its manifest version. A leading `v` is stripped so the value stays plain `X.Y.Z` for the compatibility check. Local images: `make docker-server MARREQ_VERSION=0.1.3`.

## Deployment provenance

Use a trusted, access-controlled release record for desired state. The product
version identifies the entire artifact set. Each component keeps its own release
version, source commit, registry digest and contract evidence when reused.

Release inputs are the component's version and exact source checkout SHA. Pass
both as `MARREQ_VERSION` and `MARREQ_GIT_SHA` to the production Docker build.
Supply a bare SemVer for label parity. The four final images expose OCI
`title`, `version`, `revision` and `source` labels under
`org.opencontainers.image`. Unversioned development builds use manifest
fallbacks at runtime and may carry `dev` and `unknown` labels. Deployment
verification requires exact release identity.

### Prepare and retain the evidence

The legacy release jobs capture the digest returned by a successful Buildx
registry push. The `published-*-provenance` workflow artifact holds the
published descriptor and the candidate contract from the same successful
compatibility gate. Retain both outside workflow artifact expiry. The recorder
validates the digest syntax and contract identity. Its caller must supply the
digest from the trusted publisher. Workflow records do not provide signed
attestations. Cloud provenance supports future publication through this same
interface.

Build a JSON lockfile with `format: "marreq.deployment-provenance.v1"`,
`variant: "server"` or `"cloud"`, and `components` keyed by the selected
backend, `frontend` and `mcp`. `product_release` is optional SemVer or null.
Each component uses these fields:

| Field | Evidence |
| --- | --- |
| `service` | Server uses `marreq-server`, `frontend`, `mcp-server`. Cloud uses `marreq-cloud`, `frontend-cloud`, `mcp-cloud`. |
| `version`, `source_sha` | Artifact SemVer and full lowercase Git SHA |
| `source_tag` (optional) | Matching component tag, such as `marreq-cloud-v0.2.3`. The caller verifies the Git tag resolves to the given SHA. |
| `image`, `digest` | Canonical GHCR repository and immutable `sha256:...` registry manifest or index digest |
| `contract.file`, `contract.sha256` | Relative retained contract JSON filename and lowercase SHA-256 of its exact bytes |

Store distinct retained filenames next to the manifest and adjust each
`contract.file` accordingly. Use the same retained bundles in the existing
`marreq.deployment-contract.v2` plan and run its compatibility verifier.
The provenance checker links the bytes and component, variant, version and
source identity. The contract verifier checks compatibility.

```sh
scripts/check-api-compatibility.sh --manifest deployment-contract.json
scripts/check-deployment-provenance --manifest deployment-provenance.json --mode validate
scripts/check-deployment-provenance --manifest deployment-provenance.json --mode inspect --compose-project marreq
```

Use digest-pinned Compose image references. Inspection queries Compose
project/service labels, container configuration and Docker's repository
digests for the running image configuration ID. It requires the configured
digest reference to appear in that image's repository digests. This supports
an OCI index when the daemon records its association with the selected local
image. A daemon that only exposes a configuration ID, or a platform digest
without the expected index association, yields `unverified`. The checker
also compares the OCI labels and fixed local HTTP diagnostics. It does not
pull images or modify containers. It trusts the local Docker daemon and
reports identity at observation time.

Exit 0 means `valid_manifest` in validate mode or `match` in inspect mode.
Exit 1 reports observed drift. Exit 2 reports invalid input or unavailable
evidence. A valid manifest alone verifies neither registry presence nor
runtime state. For older images retain only evidence traceable to a trusted
publication record and verifiable digest. Treat artifacts with missing
evidence as unavailable until a new verified build exists.

The future orchestrator supplies component versions from `versions.toml`,
checks source-tag targets, records successful pushes, and assembles the
deployment manifest from new and retained descriptors. It can use the
existing contract verifier before deployment and inspect the deployed
containers after replacement.

## Component build inputs and release impact (#423)

The backend images share one Dockerfile, one Cargo workspace and one lockfile.
Both builds resolve all workspace manifests. Cargo discovers placeholder
targets for the inactive backend and the contract tool. Those placeholders
fail compilation if a production dependency starts using either package.
After locked dependency fetching, the builder copies the selected backend's
`src/`, shared core, and migration tool. The runtime stage uses the selected
backend's Rocket configuration. The existing image CI matrix removes the
inactive backend's implementation and configuration and the contract tool's
sources from the build context before building each variant. The migration
startup and HTTP smoke checks then exercise those same images.

| Inputs | Image impact | Other impact |
| --- | --- | --- |
| Server or Cloud source and Rocket config | Corresponding backend | Rust and API compatibility |
| Core source and backend compatibility metadata | Server and Cloud | Rust and API compatibility |
| Workspace member manifests, root Cargo manifest and lockfile | Server and Cloud | Rust, API compatibility and database migrations |
| Backend Dockerfile, its ignore file and runtime entrypoint | Server and Cloud | Database migration and startup checks |
| Core migration SQL, schema, operational scripts and migration CLI | Server and Cloud | Database migrations, plus Rust and applicable API checks |
| Frontend source, local npm files, Vite inputs, nginx and startup configuration | Frontend | Frontend and API compatibility |
| MCP source, local npm files, Dockerfile, compatibility metadata and `MARREQ_SKILL.md` | MCP | MCP and API compatibility |
| Root Docker ignore file | Frontend and MCP | Consumer build checks |
| OpenAPI, contract analysis and code generation tooling, root npm files | None | API compatibility, plus frontend tooling for root npm files |
| Committed consumer generated types | Owning consumer | Consumer and API compatibility |
| Test files within copied source trees, and Frontend files copied alongside them | Owning image(s) | Relevant validation |
| Rust integration tests, MCP Vitest config and the released migration ledger | None | Relevant validation |
| Compose and environment example | None | Deployment validation |
| Release publishing workflow | All four conservatively | Release infrastructure and API compatibility |
| Standalone documentation | None | None |

The producer OpenAPI is bundled and consumer types are checked for drift in CI.
The image builds consume committed consumer types and their local npm lockfiles.
Changes to generator inputs call for contract validation. The generator's
checked-in output is tracked independently. MCP's checked-in `dist/` is
superseded by TypeScript output in the image. Test modules in copied Rust,
migration, Frontend and MCP source trees count as artifact inputs. Rust can
embed any copied file, and a production import can bring a TypeScript test
module into a build even when the TypeScript config excludes it from the
initial file set. Frontend's Dockerfile copies the entire project, so its
Vitest config and README also count. Rust integration tests and MCP's Vitest
config stay validation-only because their production images exclude them.

### Comparing source revisions

Run the detector from a checkout of the candidate tooling with Git and Python 3.

```sh
scripts/detect-release-impact --base "$published_source_sha" --head "$candidate_sha" --format json
```

Schema version 1 reports `base_sha`, `head_sha`, `artifacts`, `checks`,
`infrastructure` and `unknown_inputs`. The three impact objects map affected
identifiers to sorted repository-relative paths. Absent identifiers have no
impact in that comparison. `api_contracts` includes producer/consumer
compatibility across Frontend and MCP. Exit code 2 signals unresolved revisions,
an invalid map, an unsupported file mode, or unclassified paths. A report with unknown paths is diagnostic
only. Default text output is for manual review.

For independent releases, resolve each component's last **published** source tag
to its immutable commit. Run the same command once per component against the same
candidate SHA. Read only that component's key in `artifacts` from its comparison.
Union the reported checks and infrastructure reasons across successful
comparisons. Treat any failed comparison as a release blocker. The caller
supplies release baselines and must establish which artifacts were published.
Git history is compared by endpoint trees, so reversions that leave identical
files produce no impact. Deletions and both paths of renames are classified.

### Maintaining the map

Update [the impact map](../scripts/release-impact-map.json) in the same change
that introduces or moves build inputs. Rules are ordered and use Python regex
full matches against Git paths. Give validation-only rules to inputs outside
the corresponding production build boundary. A rule with only `paths` documents inputs
with no impact. Keep rules narrow for shared directories and review every
new build dependency, generator, embedded asset and workflow. A map change
needs review against the actual Docker `COPY` inputs, package graphs and CI
commands. CI checks classification coverage of the complete candidate tree,
checks file modes across the candidate tree and on changed baseline paths,
and classifies deleted paths. Only regular and executable files are supported. Historic paths can need explicit rules, as with the old single-file
OpenAPI document.

Run `python3 scripts/test-release-impact.py -v` to reproduce the acceptance
scenarios in temporary repositories. The suite covers MCP-only, Server-only,
shared core, validation-only contracts, unknown input failures, and
different release baselines. The PR job compares the merge candidate with
its first parent and attaches `release-impact.json` for inspection. Existing
CI gates continue to run independently. Build arguments such as release
version and source SHA, mutable base images, and other external inputs must
be handled by the caller. This map describes committed source changes.

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
Bearer scopes, authorization policies and stable CSRF policy IDs are checked
separately; unused global security schemes and exact global status/header set
equality do not couple APIs.
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
To provision `MARREQ_RELEASE_CONTRACTS_SHA`, export each bundle from its actual
clean release source, retain the files together, and write `deployment.json` with
their printed SHA-256 values, versions and source SHAs. Run `verify --manifest`
locally, commit that plan and its bundles, then set the repository Actions variable
to that immutable commit SHA before pushing the component tag. For a candidate,
the plan must reference the exact tagged source-exported bundle. Do not reuse a
previous plan unchanged or synthesize a manifest from current `main`; missing
historical evidence deliberately prevents publication. No global release migration
or production deployment is performed here.

A document-only check is not a release-readiness assertion. Publication also
requires passing consumer/handler tests and a provisioned immutable plan with
genuine historical bundles. Missing release evidence must continue to fail closed.

Editable contract export requires root `npm ci` for the lockfile-pinned offline
Redocly bundler. All `docs/developer/openapi/**` modules, the bundler script and
root npm manifests/lockfile and Docker build/proxy inputs must be committed alongside the Rust tool and component
inputs before export. The disposable `openapi.yaml` output is ignored and never
substitutes for these inputs. Retained bundle verification uses self-contained
JSON with the pinned standards engine; it never resolves editable source refs.
Frontend/MCP tags do not compile backend route tests in the release gate; backend
tags verify producer routes/serializers, and PR CI checks all production images.
