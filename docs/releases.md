# Marreq releases

The database safety prerequisite of #423 uses [controlled migration jobs and coherent recovery bundles](developer/database-migrations.md). Both backend images share the same serialized Diesel runner; startup only validates history. Production deployment must stop progression on backup, restore rehearsal, migration, schema or real application smoke failure. Retained binaries need an explicitly tested expansion compatibility pin; database rollback is never automatic.

[`versions.toml`](../versions.toml) is the authoritative registry. `marreq.version`
identifies the coordinated product release. The four values in `[components]`
identify independently deployable artifacts. A product release can advance
without rebuilding any images, for example to consolidate documentation changes.
This registry accepts stable SemVer (`X.Y.Z`, without leading zeroes, prerelease
suffixes or build metadata) within npm's safe integer range. Until Marreq
explicitly adopts a 1.0 compatibility policy, the coordinated product version
must remain `0.x.y`; components retain independent SemVer versions.

| Registry key | Development package version | Changelog | Compatibility declaration |
| --- | --- | --- | --- |
| `server` | `marreq-server/Cargo.toml` | [`marreq-server/CHANGELOG.md`](../marreq-server/CHANGELOG.md) | `marreq-core/frontend_compatibility.json` |
| `cloud` | `marreq-cloud/Cargo.toml` | [`marreq-cloud/CHANGELOG.md`](../marreq-cloud/CHANGELOG.md) | `marreq-core/frontend_compatibility.json` |
| `frontend` | `frontend/package.json` | [`frontend/CHANGELOG.md`](../frontend/CHANGELOG.md) | `frontend/compatibility.json` |
| `mcp` | `mcp-server/package.json` | [`mcp-server/CHANGELOG.md`](../mcp-server/CHANGELOG.md) | `mcp-server/compatibility.json` |

The [root changelog](../CHANGELOG.md) records coordinated product changes.
Each component owns its independent release notes in the changelog above.
The package fields mirror the registry for development builds. Release builds
take explicit component versions and the exact source SHA. Shared `marreq-core`,
the Rust contract and migration tools, and the root npm build tooling keep their
own package versions. The migration tool can ship inside a backend image and
inherits that image's deployment identity.

## Prepare version changes

1. Use the impact detector to identify changed artifact inputs. Compare each
   component against its own last verified publication when planning a release.
   Shared core, Cargo dependency and backend build inputs can affect both
   backends. Contract specification changes can require API checks while
   leaving all four images eligible for reuse.
2. Advance the product version for a coordinated release. Bump the versions
   of affected components and keep the version of each reused image.
3. Keep `## [Unreleased]` first in each changelog and record pending work
   there. When preparing a version, add a descending `## [X.Y.Z] - YYYY-MM-DD`
   entry with notes scoped to that product or component version. Use the
   intended release date in the candidate and the publisher's record for
   evidence of the actual publication date. `Bootstrap` headings establish
   version floors. Published and bootstrap entries keep their dates and notes.
4. Synchronize the Cargo package entries and npm manifest and lockfile version
   fields. Commit the registry, affected changelogs and generated metadata
   together.

```sh
cargo run --locked -p marreq-release -- sync
cargo run --locked -p marreq-release -- check
cargo run --locked -p marreq-release -- sync --check
cargo run --locked -p marreq-release -- --format json check --base HEAD^1 --head HEAD
```

Run the Rust CLI from a Git worktree using Cargo or build it with
`cargo build --locked -p marreq-release` and invoke `target/debug/marreq-release`
with the same arguments. `scripts/validate-release-versions` is a shell wrapper
for existing callers. The validator and impact detector use Rust and Git.
The existing deployment provenance checker consumes the two variant lockfiles
attached to every coordinated release.

Synchronization validates all inputs before staging atomic file replacements.
It changes only mirrored version values and preserves formatting, dependency
pins and unrelated lockfile entries. Rerunning sync completes an interrupted
update. `check` and `sync --check` are read-only. PR CI compares the committed
merge candidate with its first parent to enforce monotonic versions and
preserve historical changelog entries. PRs can accumulate source changes under
Unreleased. Release candidates require the appropriate bump and dated entry for
changes accumulated since each component's publication.

The impact detector classifies the registry, changelogs and validator as release
pipeline inputs. After checking both trees it excludes mirrored version-only
edits from artifact inputs. This avoids selecting Cloud when a Server version
update changes the shared Cargo lockfile. Dependency, build configuration and
file mode changes retain their impact.

## Historical bootstrap

The audit used `main` at `291c6a5b4c594dc6d754565b1c7d2a9e9dc3ec54`.
All four deployable package manifests used `0.1.0`. The coordinated product
registry starts at `0.2.0`. The existing lightweight `v1.0.0` tag at
`14348c644b302229f680ce2f23093f715c029e6b` predates coordinated product
releases and is not the version floor. The release validator exempts only this
exact historical tag at its audited commit from product-tag monotonicity;
retargeting it or introducing other future-version tags is rejected. The
release list has no coordinated product release at that tag. The canonical root changelog keeps
the original Unreleased notes and archives the `0.1.0` entry verbatim,
including its original `05/03/2026` heading. The validator recognizes that
specific archive section after registry-tracked releases. It can read the
former changelog layout from trusted historical Git baselines so the move
preserves dates, notes and the bootstrap commit's identity.

| Component | Registry floor | Highest observed tag and source commit | GHCR manifest index digest |
| --- | --- | --- | --- |
| Server | `0.2.1` | `marreq-server-v0.2.1` at `99da0ede75442549a379a36b6225f161a328128e` | `sha256:3fe12332c5f9efa905b7427291178f77521d529c7c7b79537cf148169fafc1a5` |
| Cloud | `0.1.0` | No Cloud Git source tag found | Anonymous pull token denied (HTTP 403) |
| Frontend | `0.2.2` | `marreq-frontend-v0.2.2` at `99da0ede75442549a379a36b6225f161a328128e` | `sha256:443523a03e57799b5a2a189851971cd8e7012b1566524f4907cf2fbca4a65d68` |
| MCP | `0.2.0` | `marreq-mcp-v0.2.0` at `97736eb47a8d26f5046071cc0619cfb237aabcf6` | `sha256:22fccfb3a64945a7323a256bb12ed0ef273d8cb1cd6084ea0dcc8ddf00225997` |

The Server and Frontend amd64 images carry OCI version and revision labels
matching their tags. The MCP publication run succeeded at the tagged revision
and GHCR serves the image, but its amd64 image config has no OCI version or
revision labels. The registry digest and Git tag are separate observations for
MCP. Cloud keeps its source version. Cloud `0.2.3` and MCP `0.2.4` are CI
image identity fixtures. The legacy `marreq-backend-v0.1.12` annotated tag
peels to `02096fa729a2065cbc9366d8717d1e20fcb8d608`. Anonymous GHCR access
to the legacy Backend repository returned HTTP 403.

The [GitHub release list](https://github.com/MarreqSW/Marreq/releases) shows
one release at `marreq-server-v0.1.5`. The successful runs for the highest
observed versions are available in the
[Server](https://github.com/MarreqSW/Marreq/actions/runs/39554111734),
[Frontend](https://github.com/MarreqSW/Marreq/actions/runs/39554112426) and
[MCP](https://github.com/MarreqSW/Marreq/actions/runs/39505636702) run records.
No `published-*-provenance` records were returned by the Actions artifact
list for any of the four names. The observed images predate the retained
provenance workflow. Their registry digests alone cannot establish trusted
deployment baselines with retained contract evidence.

To reproduce the audit, list Git tags and peel them with
`git rev-parse "refs/tags/$tag^{commit}"`. Check
`GET /repos/MarreqSW/Marreq/releases` and
`GET /repos/MarreqSW/Marreq/actions/artifacts?name=published-server-provenance`
on the GitHub API, repeating the artifact query for the other components.
For each GHCR repository request a pull token with scope
`repository:marreqsw/<image>:pull`, list `/v2/marreqsw/<image>/tags/list` and
read `Docker-Content-Digest` from a GET of
`/v2/marreqsw/<image>/manifests/<version>` with OCI index and Docker manifest
list media types accepted. Follow the amd64 child manifest to its config blob
for OCI labels. Audit digests were taken from explicit version tags.

## Validate and tag a release

Complete the version and changelog changes on a reviewed PR. After merging,
fetch all tags and use the current tip of protected `main`. Queue one coordinated
tag at a time and keep `main` at the candidate until a publication journal exists.

```sh
git fetch origin main --tags
git switch --detach origin/main
candidate_sha=$(git rev-parse HEAD)
product_tag=v0.2.1 # use the exact marreq.version from versions.toml
cargo run --locked -p marreq-release -- check
cargo run --locked -p marreq-release -- sync --check
cargo run --locked -p marreq-release -- tag "$product_tag" --head "$candidate_sha"
git tag -a "$product_tag" "$candidate_sha" -m "Marreq ${product_tag#v}"
cargo run --locked -p marreq-release -- --format json release-base \
  --head "$candidate_sha" --tag "$product_tag"
```

Use the reported `previous` SHA. For a subsequent release, download the
prior GitHub Release's variant provenance and contract assets together into
`prior/`. Run the same planner used by GitHub Actions.

```sh
cargo run --locked -p marreq-release -- --format json release --publication \
  --head "$candidate_sha" --tag "$product_tag" --previous "$previous_sha" \
  --server-provenance prior/server-provenance.json \
  --cloud-provenance prior/cloud-provenance.json
git push origin "refs/tags/$product_tag"
```

The planner returns the candidate SHA, versions, changed input paths, and all
four build or reuse decisions. It requires the annotation and canonical
prior product tag in publication mode. The standalone `tag` command can run
before creating the annotation and checks the proposed name against the registry.
Actions verifies the pushed tag object and the protected branch independently.

### First coordinated publication

The audited historical images above lack trusted retained contracts. For the
first coordinated release, advance the product version and all four component
versions above their historical floors and add one dated changelog entry for
each. With the current floors, examples of fresh slots are Server `0.2.2`,
Cloud `0.1.1`, Frontend `0.2.3`, and MCP `0.2.1`. Confirm availability and
compatibility for the actual candidate versions. Run `sync` before committing.
Create the annotated product tag locally as above. `release-base` reports the
registry introduction commit as `previous` and a null prior tag.

```sh
cargo run --locked -p marreq-release -- --format json release --publication \
  --bootstrap-publication --head "$candidate_sha" --tag "$product_tag" \
  --previous "$previous_sha"
```

Set the repository Actions variable `MARREQ_BOOTSTRAP_TAG` to the exact product
tag before pushing it. This opt-in only works when the canonical tag history
has no preceding product release. The planner requires a new dated version for
every component, keeps the historical floor and the audited `v1.0.0` exception,
and applies the normal impact, compatibility and publication gates. The Cloud
staging push creates its package if needed. The publisher verifies Cloud's
version slot once the namespace is readable and before creating a release
draft or version alias. Clear the variable after the first successful release.
Later releases require genuine evidence from the preceding immutable release.

### GitHub and GHCR configuration

- Protect `main` with required reviewed PRs and CI, dismiss stale approvals,
  and restrict direct and force pushes. Require code-owner approval for changes
  to workflows, the release tooling and its scripts, and the release impact
  map. Protect those review rules as well. The workflow checks that GitHub
  reports `main` protected and initially requires the tagged commit to be its
  tip.
- Restrict creation of `v*` tags to the release maintainer team. Apply a separate
  active ruleset that prevents modification and deletion of product tags,
  including the historical tag. A release maintainer should push only the
  locally validated annotated tag on the reviewed candidate.
- Protect the four `marreq-{server,cloud,frontend,mcp}-v*` source tag patterns
  with a separate no-update/no-delete ruleset. For creation restrictions, grant
  bypass only to the publisher identity. Where the default Actions token cannot
  be granted this bypass, configure `MARREQ_SOURCE_TAG_TOKEN` in the publication
  environment as a dedicated release service account's expiring fine-grained
  token for this repository with **Contents write**. Grant the account's team
  bypass on the component-tag creation ruleset only. The workflow uses this
  credential only for create-ref calls. Component tags point at the coordinated
  workflow revision, whose trigger matches product tags exclusively.
- Enable [immutable releases](https://docs.github.com/en/repositories/releasing-projects-on-github/managing-releases-in-a-repository).
  Create the `release-publication` environment with required reviewers,
  self-approval disabled and administrator bypass disabled. Restrict it to the
  product tag pattern. Set `MARREQ_RELEASE_POLICY_TOKEN` there to an expiring
  fine-grained token scoped to this repository with **Administration read**.
  The publisher uses it to verify the immutable-release setting before staging
  and again before finalization. Keep the repository workflow default read-only.
  Reviewers should confirm the run used the reviewed tag and passed both
  read-only validation jobs before granting publication.
- Grant the Marreq repository Actions access to the four canonical GHCR packages
  in the table below, and permit package creation for Cloud at bootstrap. Limit
  package writes to the repository's protected publisher. Remove legacy
  publishers and shared package-write credentials. GHCR lacks conditional tag
  creation, so the no-overwrite guarantee relies on exclusive package writers
  together with the workflow's repository-wide concurrency group. Retain all
  published digests and the package access required for reuse.

### Publication lifecycle

1. The product-tag push workflow verifies the annotation's direct commit
   target, event identity, and current protected `main`. Read-only jobs validate
   that exact source and the registry and changelogs, run Rust and consumer tests
   and builds, validate generated API types and exercise both backend variants
   over HTTP. They run RustSec and production npm audits using the repository's
   existing audit thresholds.
2. The planner resolves the newest preceding canonical product tag. It requires
   an immutable GitHub Release authored by the publisher and downloads the
   variant provenance and retained contracts, checking each asset's digest.
   The planner verifies Git ancestry, component tags, version history and the
   dependency-aware impact graph. A new component uses its candidate contract.
   A reused component keeps its prior source and contract bytes. Both complete
   Server and Cloud compatibility plans must pass the existing contract verifier.
3. The protected publication job checks version slots and reused artifacts.
   It builds only the selected new components from an archive of the validated
   commit. Extraction uses GNU tar `--same-permissions` so the publisher's
   `umask 077` still protects credentials and journals without stripping
   archived file modes that non-root container entrypoints need. Buildx keeps
   its cache across the selected component builds in the same release and
   pushes separate run-attempt staging aliases. The publisher checks actual
   registry digests, pulls by digest, compares OCI source, version and
   contract-hash labels, and runs disposable runtime identity smoke checks on
   each newly staged image.
4. A private draft GitHub Release stores the product manifest and all evidence.
   The manifest also records the digests for recovery. After verifying the
   uploaded bytes, the publisher assigns exact existing digests to the new
   version aliases, verifies them, and creates the new source tags. Existing
   matching aliases and source tags on a retry are read-only. Every reuse keeps
   its original source tag and digest.
5. Once both compatibility gates, all digests, source tags and assets are
   verified, the publisher finalizes the draft as an immutable GitHub Release.
   Release notes come from the product entry and the newly published
   components' changelog entries.

| Component | Source tag | Canonical GHCR repository |
| --- | --- | --- |
| Server | `marreq-server-vX.Y.Z` | `ghcr.io/marreqsw/marreq-server` |
| Cloud | `marreq-cloud-vX.Y.Z` | `ghcr.io/marreqsw/marreq-cloud` |
| Frontend | `marreq-frontend-vX.Y.Z` | `ghcr.io/marreqsw/marreq-frontend` |
| MCP | `marreq-mcp-vX.Y.Z` | `ghcr.io/marreqsw/marreq-mcp-server` |

### Inspect release evidence

`release-manifest.json` uses schema version 1 and records the product tag
object and source SHA, UTC journal time as Unix seconds, publisher run ID,
baseline, and both compatibility and deployment provenance hashes. Its four
component entries include `build` or `reuse`, version, source tag and commit,
canonical image, digest-pinned `reference`, and retained contract identity.
Use the `reference` value as the deployment image reference. The
`server-provenance.json` and `cloud-provenance.json` lockfiles follow the existing
deployment schema.

```sh
gh release download "$product_tag" --repo MarreqSW/Marreq --dir release-evidence
jq '.components | map_values({version, source_sha, reference, action})' \
  release-evidence/release-manifest.json
scripts/check-api-compatibility.sh --manifest release-evidence/server-compatibility.json
scripts/check-api-compatibility.sh --manifest release-evidence/cloud-compatibility.json
scripts/check-deployment-provenance --manifest release-evidence/cloud-provenance.json --mode validate
```

The compatibility verifier names the rejected variant or consumer and the
incompatible version, route, schema, or policy. Confirm the referenced source
SHAs and hashes in the selected plan before changing the candidate. Update a
consumer or producer and its compatibility policy and changelog as needed
through a reviewed PR. Repeat the checks against both selected combinations.

### Recover a failed publication

Use **Re-run failed jobs** on the original workflow run. Before the manifest
has been retained, no component version tags or aliases have been created.
A retry can build to fresh run-attempt staging aliases while the candidate is
still the protected `main` tip. After the draft manifest exists, the same run
uses its recorded digests and restores any missing draft assets. A journaled
candidate can finish after `main` advances if it remains on protected
`main`'s first-parent history. Keep the validated Actions artifact available
until completion so the publisher can compare the journal against the original
plan.

An image push may succeed before a later promotion, source-tag creation or
finalization fails. Leave the draft, product tag, staged digests and any
assigned version aliases in place. A retry accepts the exact recorded digest
and expected tag target, and stops on a missing digest, a conflicting slot,
a different run ID or changed evidence. Investigate package access, runner and
registry errors before retrying. If an unexpected image or source tag occupies
a slot, treat it as a release incident. Preserve the draft, run logs and
registry records for maintainer review; publication will not overwrite the
slot. Keep the draft private until the workflow verifies and finalizes it.
Only retire unneeded staging aliases using registry procedures that preserve
every referenced digest and its version alias.

If the candidate itself must be replaced before a journal was retained, first
confirm that no public release, journal, component version alias or component
source tag was created by its run. The orphan product tag blocks subsequent
candidates. Have a repository administrator record the abandonment and use the
ruleset's controlled emergency procedure to remove that unpublished product
tag. Prepare and merge a corrected candidate and use a new product version.
Reserve this procedure for unpublished candidates with no committed image
identity. A journaled candidate requires incident review if it cannot resume.

### Automatic production deployment

After `publish` succeeds, the same workflow runs a `deploy` job against the
GitHub Environment `production`. It SSHes to the VPS with a restricted
forced-command identity and runs `scripts/deploy-release.sh` for the product
tag. Only digest-changed Server/Frontend/MCP services are updated. PostgreSQL
and Umami are left untouched. Pending migrations fail closed for operator
recovery; see [continuous deployment](developer/continuous-deployment.md).

Manual redeploy or schema-aware application rollback of an already published
tag uses **Deploy Marreq production** (`deploy-production.yml`, optional
dry-run). Component tags never deploy. Do not trigger a new product tag solely
to test CD.


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

Widen or tighten the compatibility declarations when introducing a breaking API or UI contract change. Server and Cloud use the same frontend contract range while versioning their images independently. Validate the selected artifact set using the retained contract checks below.

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

The coordinated release retains both variant provenance lockfiles and the
exact selected contracts as immutable GitHub Release assets. The publisher
checks the digest returned by Buildx against the registry before retaining it
and compares the contract's byte hash with the OCI image label.

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
| Workspace member manifests, root Cargo manifest and lockfile | Server and Cloud for shared build inputs | Rust, API compatibility and database migrations |
| Version registry, product and component changelogs, and release CLI | None | Release pipeline |
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
| Release workflow and its GHCR build script | All four conservatively | Release infrastructure and API compatibility |
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

Run the Rust detector from a checkout of the candidate tooling with Git.

```sh
cargo run --locked -p marreq-release -- impact --base "$published_source_sha" --head "$candidate_sha" --format json
```

`scripts/detect-release-impact` delegates to the same command. The backend
Docker build supplies a placeholder for the release tool so Cargo can resolve
the locked workspace. Adding just that workspace member, its isolated lock
entry and the exact Docker placeholder wiring has no artifact impact. Other
manifest, lockfile and Docker changes are classified normally.

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
that introduces or moves build inputs. Rules are ordered and use Rust regex
full matches against Git paths. Give validation-only rules to inputs outside
the corresponding production build boundary. A rule with only `paths` documents inputs
with no impact. Keep rules narrow for shared directories and review every
new build dependency, generator, embedded asset and workflow. A map change
needs review against the actual Docker `COPY` inputs, package graphs and CI
commands. CI checks classification coverage of the complete candidate tree,
checks file modes across the candidate tree and on changed baseline paths,
and classifies deleted paths. Only regular and executable files are supported. Historic paths can need explicit rules, as with the old single-file
OpenAPI document.

Run `cargo test --locked -p marreq-release` for the release CLI integration
tests. Run `python3 scripts/test-release-impact.py -v` for the existing
impact CLI and shell wrapper regression tests in temporary repositories. The suite covers MCP-only, Server-only,
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
the bundle/source to the actual image digest. The coordinated publisher binds the retained contract to the image digest
and records both in the product manifest.

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

The coordinated release gate checks both selected deployment plans. It uses the
candidate bundles for new images and exact retained bundles for reused images.
PR CI checks calls from the current source tree. Release CI also exercises the
current source against both backend variants before publication.

Editable contract export requires root `npm ci` for the lockfile-pinned offline
Redocly bundler. All `docs/developer/openapi/**` modules, the bundler script and
root npm manifests/lockfile and Docker build/proxy inputs must be committed alongside the Rust tool and component
inputs before export. The disposable `openapi.yaml` output is ignored and never
substitutes for these inputs. Retained bundle verification uses self-contained
JSON with the pinned standards engine; it never resolves editable source refs.
