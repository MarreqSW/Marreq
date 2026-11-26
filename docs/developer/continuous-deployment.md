# Continuous deployment

Marreq deploys automatically to the existing single production VPS after every
successful coordinated product release. Publication remains the immutable gate;
deployment is a selective Docker Compose update over SSH.

See also [releases](../releases.md) and [database migrations](database-migrations.md).

## Production architecture

| Item | Value |
| --- | --- |
| Public URL | `https://app.marreq.com` |
| Host role | Ubuntu VPS (~1 vCPU, 1 GiB RAM) |
| SSH user | `marreq` |
| Repository checkout | `/home/marreq/Marreq` |
| Compose project | `docker` |
| Reverse proxy | Caddy (HTTPS) terminating to loopback frontend |
| Analytics | Umami in a separate Compose project sharing PostgreSQL |

Application containers (Server variant, no Ollama):

| Container | Service |
| --- | --- |
| `docker-marreq-server-1` | `marreq-server` |
| `docker-frontend-1` | `frontend` |
| `docker-mcp-server-1` | `mcp-server` |
| `docker-db-1` | PostgreSQL 17 + pgvector |
| `docker-adminer-1` | Adminer (optional) |

Production Compose uses three files. The first two are operator-maintained on
the VPS and must not be overwritten by `git pull` defaults:

1. `docker/docker-compose.light.github.yml` — GHCR-oriented light stack base
2. `docker/docker-compose.persistence.yml` — external volume pins
3. `docker/docker-compose.release.yml` — digest-pinned app images (generated)

### Persistent volumes

PostgreSQL and attachments use external volumes. The long PostgreSQL volume name
is intentional (former anonymous volume). Never rename, prune, or recreate these
volumes during ordinary deployments.

Attachment files are owned by UID/GID `10001:10001`.

Expected PostgreSQL cluster identity for this host is recorded in
`MARREQ_EXPECTED_PG_CLUSTER_ID` (default `7690530505020129316`).

## Lifecycle

```text
Push product tag vX.Y.Z
        |
        v
Existing release pipeline (source → rust → plan → publish)
        |
        v
Immutable GHCR images + GitHub Release
        |
        v
deploy job (environment: production)
        |
        v
SSH forced-command → scripts/deploy-release.sh
        |
        v
Selective compose update + health checks
```

Component source tags (`marreq-*-v*`) do not trigger deployment. Pull requests
never receive production credentials.

## GitHub Environment `production`

| Name | Kind | Purpose |
| --- | --- | --- |
| `DEPLOY_SSH_HOST` | variable | Hostname (`app.marreq.com`) |
| `DEPLOY_SSH_PORT` | variable | SSH port (`22`) |
| `DEPLOY_SSH_USER` | variable | `marreq` |
| `DEPLOY_SSH_KNOWN_HOSTS` | variable | Verified `known_hosts` lines |
| `DEPLOY_PUBLIC_URL` | variable | `https://app.marreq.com` |
| `DEPLOY_SSH_PRIVATE_KEY` | secret | Actions-only deploy private key |

The environment uses a custom deployment-branch policy that allows coordinated
product tags (`v*.*.*`) and `main` (for the manual `deploy-production.yml`
workflow). Production deployments do not require a second manual approval after
`release-publication` succeeds. Do not weaken `release-publication` protections.

### SSH deploy identity

1. Generate an unencrypted ed25519 key used only by GitHub Actions.
2. Install the public key on the VPS with a forced command:

```text
command="/home/marreq/bin/marreq-deploy-entry",no-agent-forwarding,no-port-forwarding,no-X11-forwarding,no-pty ssh-ed25519 AAAA... github-actions-marreq-production-deploy
```

3. Store the private key as `DEPLOY_SSH_PRIVATE_KEY`.
4. Verify the host key out of band and put the exact lines in
   `DEPLOY_SSH_KNOWN_HOSTS`. Never disable `StrictHostKeyChecking`.

Operator admin keys remain separate from the Actions deploy key. Docker access
is root-equivalent; keep the forced command allowlist tight.

Allowed remote commands (via [`scripts/production-deploy-entry.sh`](../../scripts/production-deploy-entry.sh)):

- `deploy vX.Y.Z`
- `deploy --dry-run vX.Y.Z`
- `rollback vX.Y.Z`
- `rollback --dry-run vX.Y.Z`

## Deployment script

[`scripts/deploy-release.sh`](../../scripts/deploy-release.sh) wraps
[`scripts/deploy_release.py`](../../scripts/deploy_release.py).

```sh
scripts/deploy-release.sh --dry-run v0.2.1
scripts/deploy-release.sh v0.2.1
scripts/deploy-release.sh --rollback v0.2.1
```

Preflight acquires `/home/marreq/.marreq-operations.lock` (shared with backups),
validates the immutable release and retained evidence, checks Compose layout,
PostgreSQL cluster identity, attachment mounts, and free disk, then rejects
stale/out-of-order tags.

Selective update reads digest-pinned `reference` values from
`release-manifest.json`, writes only `docker-compose.release.yml`, pulls changed
images, and recreates one service at a time:

```sh
docker compose -p docker \
  -f docker/docker-compose.light.github.yml \
  -f docker/docker-compose.persistence.yml \
  -f docker/docker-compose.release.yml \
  up -d --no-deps --no-build --pull never --wait SERVICE
```

Identical installed digests are a no-op. The script never builds on the VPS,
never uses `latest`, and never recreates `db`, Adminer, or Umami during ordinary
application deployments.

Installed state is retained under `releases/current/` with the previous verified
override under `releases/previous/`.

### Migrations

When the server digest changes, the candidate image runs `marreq-migrations check`
against the live database. No pending migrations → deploy continues.

Pending migrations → deployment stops before mutation. On this 1 GiB host,
automatic coherent recovery (quiesce → `marreq-backup` → isolated restore
rehearsal → migrate) is an operator procedure. Follow
[database migrations](database-migrations.md), then re-run deploy.

Never use `dev-init` in production. Never auto-restore PostgreSQL or run
`down.sql`.

### Health checks

Success requires running containers, runtime identity matches for backend
(`/api/meta/build`), frontend (`build-info.json`), and MCP (`/build-info`),
unchanged PostgreSQL cluster identity, correct volume mounts, and HTTP 200 from
`${DEPLOY_PUBLIC_URL}/login`. SPA 200 alone is not sufficient; backend identity
must match the release manifest.

Authenticated smoke tests run only when a local non-git credential file is
configured by the operator. Missing credentials are reported; customer data is
never created solely to test a deploy.

## Manual recovery and redeploy

Use **Actions → Deploy Marreq production → Run workflow**
(`.github/workflows/deploy-production.yml`). Manual redeploy is intentionally
separate from the coordinated release workflow so `workflow_dispatch` cannot
share cache-writing release jobs.

| Input | Meaning |
| --- | --- |
| `tag` | Existing published `vX.Y.Z` |
| `action` | `deploy` or `rollback` |
| `dry_run` | Validate without mutation |

Rollback restores the previous retained image references only when compatible
with the resulting schema. It never reverses migrations or restores database
dumps.

If deployment fails after a migration that was applied manually, stop, preserve
recovery evidence, prefer safe forward repair, and require explicit operator
approval before any data-loss recovery.

## Backups

Routine online backups use
[`scripts/marreq-production-backup.sh`](../../scripts/marreq-production-backup.sh).

Install on the VPS:

```sh
scripts/install-production-backup.sh --run-now
```

Defaults:

| Setting | Value |
| --- | --- |
| Schedule | 03:00 local (systemd user timer, cron fallback) |
| Location | `/home/marreq/backups/marreq/` |
| Contents | `marreq` + `umami` dumps, PostgreSQL globals, attachments |
| Checksums | SHA-256 generated and verified before publish |
| Lock | `/home/marreq/.marreq-operations.lock` |
| Retention | 7 daily + 4 weekly |

These online backups are **not** coherent pre-migration recovery bundles.
Application writes may continue during dump/copy. For schema releases, stop
writers and use `marreq-backup`.

### Off-site storage

Set `MARREQ_BACKUP_OFFSITE_CMD` to a local wrapper that receives the published
backup directory path. If unset, the script logs that off-site storage is not
configured and keeps local retention only. Do not invent credentials or send
production data to an unauthorized destination.

## Initial VPS setup checklist

1. Preserve local Compose customizations (Umami nginx mounts, `.env`, Caddy).
2. Confirm persistence override and cluster identity.
3. Install repository scripts (or sync only the `scripts/` tree without
   overwriting local Compose files).
4. Install `scripts/production-deploy-entry.sh` as
   `/home/marreq/bin/marreq-deploy-entry`.
5. Authorize the Actions deploy public key with the forced-command options above.
6. Install and verify the backup timer (`--run-now`).
7. Dry-run and no-op deploy the currently installed product tag.

## Prohibited operations

Never run against production:

- `docker compose down -v`
- `docker volume prune` / `docker system prune --volumes`
- Database reset/seed/`dev-init`
- Automatic destructive migrations or down migrations
- Automatic database restoration
- Uncontrolled PostgreSQL image upgrades
- Recreating PostgreSQL during ordinary app deploys
- Removing existing production volumes to reclaim disk

## Troubleshooting

| Symptom | Check |
| --- | --- |
| Deploy blocked by lock | Backup still running; wait or inspect `.marreq-operations.lock` |
| Stale deploy rejected | Requested tag older than `releases/current/installed-state.json` |
| Cluster identity mismatch | Wrong volume mounted; stop and investigate before any write |
| Pending migrations | Perform operator recovery gate, then redeploy |
| Provenance unverified | Runtime checks may still pass; inspect RepoDigests vs digests |
| SSH denied | Forced-command key vs admin key; known_hosts mismatch |

## Outstanding limitations

- Single small VPS: sequential service updates; no staging environment.
- Pending migrations are fail-closed; not auto-applied by CD.
- Offline/off-site backup requires operator-supplied `MARREQ_BACKUP_OFFSITE_CMD`.
- Authenticated end-to-end smoke needs operator-provided non-destructive
  credentials on the host.
- Production Compose base/persistence files remain host-local and are not
  replaced by repository defaults.
