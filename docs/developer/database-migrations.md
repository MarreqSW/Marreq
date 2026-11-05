# Controlled PostgreSQL schema releases (#423)

Backend startup never migrates or seeds. Both Server and Cloud validate schema
history before binding a socket, including native `cargo run` binaries. Prepare
the database explicitly. A restart is a read-only schema operation.

## Architecture and audit

Previously the shared Docker entrypoint waited indefinitely, ran `diesel migration
run`, and defaulted demo seeding to true. Development Compose and `db_setup.sh` /
`db_migrate.sh` used separate direct Diesel commands. Application initialization
itself did not migrate. API-contract smoke setup previously executed SQL files
without recording Diesel history; it now uses the same runner. Other PostgreSQL
integration fixtures intentionally build isolated test schemas directly.

`marreq-migrations` uses Diesel's existing embedded migrations, per-migration
transactions (unless `metadata.toml` explicitly disables them), and
`__diesel_schema_migrations`. It holds PostgreSQL session advisory lock
`0x4d4152524551` on **the same connection that executes Diesel migrations**. The
lock covers history validation, recovery evidence validation, execution and
postflight. Independent Server/Cloud jobs share this database-scoped key.
`MARREQ_MIGRATION_LOCK_SECONDS` defaults to 60 (0–3600 permitted). Connection
closure releases the lock, including process/container failure; PostgreSQL may
finish/cancel an in-flight statement before observing a dead client. Statements
are bounded to 15 minutes and DDL lock acquisition to 5 seconds. A stopped process
can still own its connection; terminate it deliberately rather than bypassing the
lock. A terminated connection never leaves a permanent advisory lock.

No extra migration state database/service exists. History must be an exact ordered
prefix of bundled identities. Unknown versions, holes, and missing history on a
nonempty public schema fail closed. Do not reconstruct history automatically.
Diesel does not checksum historical SQL: committed checksums freeze the 30
existing migrations in CI. Never edit/reorder/squash released files. All historical
migrations remain unchanged, including their existing bootstrap users. Production
`apply` blocks release progression while the exact historical public bootstrap
password hash is enabled. After fresh initialization, provision a secure admin and
explicitly rotate/disable that bootstrap credential, then rerun `apply`. This guard
also runs on no-pending production reruns; it is not a general password-strength
audit. Keep traffic closed until remediation and smoke checks succeed.

Only supported runner commands serialize; direct Diesel/psql writes can bypass
advisory locks. Restrict the migration owner credential to operators/jobs; ordinary
backend roles should have only required DML and sequence privileges, no CREATE or
ownership. Grant table/sequence/default privileges for both current and future
objects. Some legacy migrations create `vector`: provision that extension through
a DBA where the owner role cannot install it. The runner conservatively checks
schema CREATE and existing table/sequence ownership before DDL; PostgreSQL remains
the authority for individual DDL permissions. No superuser runtime is required.

## Development

Export `DATABASE_URL` (or use a private `.env`), then from the repository:

```sh
cargo run --locked -p marreq-migrations -- dev-init
cargo run --locked -p marreq-migrations -- check
cargo run -p marreq-server
```

`marreq-core/scripts/db_setup.sh [--seed]` still creates a local database and now
uses `dev-init`; development Compose does likewise before `cargo run`.
`db_migrate.sh dev-init` is the equivalent wrapper. Demo seeding is an explicit
`db_seed.sh` operation; `MARREQ_AUTO_SEED` defaults to false and true is rejected
by the container entrypoint. Production Compose also pins false.

`dev-init` is explicitly for disposable/local development and skips the recovery
gate. Never use it to evade production preflight. Runtime images contain the
runner and existing backup client, with no Diesel CLI. The builder retains Diesel
CLI for generating migrations:

```sh
cd marreq-core
diesel migration generate add_project_description_hint
```

Write `up.sql`, `down.sql`, and the review metadata described below, then use the
runner. Direct `diesel migration revert/redo` is a destructive development-only
manual operation and does not participate in the orchestration lock.

`DATABASE_URL` and Rocket's `databases.my_db.url` must agree. Set
`ROCKET_DATABASES='{my_db={url="..."}}'` when overriding the native default.
Passwords must be provided through private environment/secret files; never paste
connection URLs into logs. Commands deliberately omit credential values.

## Expand → transition → contract

For example, replace `projects.description` with `description_hint`:

1. **Expand:** add nullable `description_hint`, leaving `description` intact.
   Retained code continues to read/write `description`.
2. **Transition:** deploy compatible code; specify which value wins, use guarded
   dual writes if necessary, and run bounded, restartable backfill batches outside
   the schema runner. Do not overwrite a concurrently updated value with a stale
   backfill. Verify both retained and candidate binaries against the expansion.
3. **Contract:** after all supported binaries and rollback targets stop using
   `description`, explicitly approve dropping it in a separate release. That is
   the rollback boundary; binaries relying on it can no longer be restored safely.

Every new directory needs `review.json`, for example:

```json
{
  "phase": "expand",
  "compatibility_window": "Retained Server/Cloud image digests ... through candidate ...",
  "lock_risk": "Nullable column, brief ACCESS EXCLUSIVE metadata lock; 5s timeout",
  "rollback_limit": "Retained binaries continue using description; do not drop it",
  "validation": "Old/new authenticated HTTP smoke and backfill fixtures ..."
}
```

`python3 scripts/check-migration-policy.py` freezes released files, rejects invalid
or duplicate/new out-of-order identities, requires review metadata, and blocks
obvious destructive/data-transforming SQL and premature NOT NULL additions.
This conservative text gate is **not a SQL safety proof**. Review DDL semantics,
size estimates, locks, defaults, triggers, and application behavior. Contract or
data-transforming migrations deliberately fail this expansion gate; they need a
separate reviewed gate change and compatibility evidence, rather than a boolean
bypass. Do not label them expand to get through CI.

Most PostgreSQL DDL is transactional but can hold strong locks until commit.
Large table rewrites/default expressions and constraint validation can block
traffic. Use `CREATE INDEX CONCURRENTLY` where appropriate; it cannot run inside a
transaction, so use Diesel `metadata.toml` with `run_in_transaction = false` after
review. A failed concurrent index build may leave an invalid index; inspect and
repair explicitly before retry. A job can commit earlier migrations before a
later one fails. Never assume all migrations are one atomic transaction.

Startup requires the binary's full known history. By default a newer schema is
also rejected. For a **tested expansion only**, pin
`MARREQ_SCHEMA_COMPATIBLE_THROUGH` to the exact maximum applied migration identity
on retained binaries, e.g. `20261105000001`. This is a trusted operator assertion,
not automatic compatibility detection. The release must retain actual old/new
image smoke evidence before setting it. No wildcard/range is accepted. Gaps in
the required known prefix still fail. The migration job itself never accepts
unknown versions, even when the application compatibility pin is set.

## Offline recovery bundle

The existing `db_backup.sh` remains a database-only gzip utility (now private,
verified, and atomically published). Admin → Backup remains the existing
`marreq.backup.v1` archive. **An online admin download is not coherent release
recovery evidence.** Database dumps and blob traversal occur at different times.

Release backups use `marreq-backup`, a small CLI around that same application
archive implementation. They require:

- Stop **all** Server/Cloud replicas, workers, admin scripts and attachment writers
  for this database. Keep them stopped from backup through migration/postflight.
  Disable their reconnect/restart/deployment automation. Restrict writer access as
  necessary. Quiescence is an explicit operator responsibility, not detected by
  the boolean `MARREQ_WRITES_QUIESCED=true`; without that assertion the tools fail.
- Supply the full existing attachment directory, even if empty. If Server/Cloud
  share a DB but use different stores, prepare a private offline union of **all**
  stores (preserving `ab/cd/hash` paths) first, or mount a shared complete store.
  Hash collisions/content corruption and missing referenced blobs fail validation.
  Include soft-deleted blobs because baselines can still reference them.
- Use the database/attachment backup owner credential, a matching PostgreSQL
  client (images supply PG17), sufficient disk space for dump/archive and isolated
  restore, and a new output directory on a private backup volume. The tool does
  not estimate free space; ENOSPC fails the command and no valid receipt is made.

On a host with the matching client and binaries (or via the candidate image):

```sh
export MARREQ_WRITES_QUIESCED=true
export MARREQ_RECOVERY_BUNDLE=/private/recovery/bundle
export MARREQ_ATTACHMENTS_DIR=/complete/offline/attachment-store
# DATABASE_URL is supplied privately; do not put the URL in shell arguments.
marreq-backup
marreq-migrations verify-backup
```

The parent recovery directory must exist and be writable; the tool creates a new
0700 bundle containing `backup.tar.gz` and `recovery.json`. It records archive
SHA256, database identity, applied versions, application/source version, quiescence
attestation, and all referenced attachment hashes/sizes. It streams the entire
archive, validates gzip/tar/manifest, rejects unsafe paths/links/duplicate entries,
checks blob contents, totals and references. It does not authenticate malicious
replacement of both archive and evidence: store the directory with trusted access,
retain it off-host securely, and apply normal backup encryption/retention policy.
A failed partial bundle must be investigated and replaced using a new directory.

For the image, mount the selected attachment volume read-only and recovery root
read-write; set `DATABASE_URL` in the calling environment, select the existing
Compose network and immutable candidate image, and run as the existing UID 10001:

```sh
# NETWORK, ATTACHMENTS_VOLUME, MARREQ_RECOVERY_ROOT and candidate image are operator inputs.
# Provision the private recovery root for UID 10001 first.
docker run --rm --network "$NETWORK" -e DATABASE_URL -e MARREQ_WRITES_QUIESCED \
  -e MARREQ_RECOVERY_BUNDLE=/recovery/bundle -e MARREQ_ATTACHMENTS_DIR=/attachments \
  --mount "type=volume,src=$ATTACHMENTS_VOLUME,dst=/attachments,readonly" \
  --mount "type=bind,src=$MARREQ_RECOVERY_ROOT,dst=/recovery" \
  --entrypoint /usr/local/bin/marreq-backup "$MARREQ_MIGRATION_IMAGE"
```

For multiple stores, use a read-only bind of the complete staged union instead of
the single named volume. The backup holds the same advisory lock as migrations.
Online backup/API behavior is unchanged; it cannot produce release evidence.

## Restore rehearsal and explicit recovery

Before applying pending production migrations, restore **this exact bundle**:

1. Provision an isolated empty PostgreSQL database with vector available, matching
   PG17+ clients, and an empty private attachment directory. Never point rehearsal
   commands at production. Dumps are trusted SQL and can execute SQL code.
2. Verify the archive first. Extract it into a new private directory. Restore
   `database.sql` with `psql -X -v ON_ERROR_STOP=1 --single-transaction`, providing
   target credentials through a private service/environment, and copy the archive's
   `attachments/` tree to the isolated store. Missing archive files are failures.
3. Set `DATABASE_URL` to the **restored** DB and `MARREQ_ATTACHMENTS_DIR` to its
   restored store. Run `marreq-migrations check` using the matching retained schema
   image if the backup predates the candidate. Start the retained application
   against it and run authenticated read/write/download smoke tests.
4. Run `marreq-migrations verify-restored`. This compares restored Diesel history
   and attachment rows with the bundle, checks restored file hashes/sizes and
   constraint validation, rejects the original database identity, and writes
   `restore-verified.json`. New receipts use create-new semantics. A receipt is
   integrity/rehearsal evidence, **not** proof of application compatibility or a
   substitute for actual HTTP smoke. Keep that evidence with deployment records.

For empty stores, create the directory explicitly even if the archive has no
attachment entries. For pre-attachment schemas, there are no attachment rows to
check. Foreign keys/check constraints are enforced during plain-SQL restore;
`verify-restored` also rejects unvalidated public constraints (review intentional
NOT VALID constraints manually rather than suppressing the check).

Never automatically restore production after migration or smoke failure. Stop
progression, inspect committed migration history, and repair forward where safe.
Application rollback means reverting image digests against an expanded compatible
schema; it does **not** run `down.sql`. If the compatibility boundary has been
crossed, restoration/manual repair is an explicit incident decision: stop writers,
preserve the failed state and new writes, assess data loss, restore database **and**
matching attachments together, validate and smoke-test, then reopen traffic.
Recovery loses writes after the backup; a database down migration cannot promise
to recover removed/transformed data.

## Production interface and future release gate

Set `MARREQ_MIGRATION_IMAGE` to the selected Server **or** Cloud candidate digest,
`MARREQ_MIGRATION_DATABASE_URL` to the protected migration owner connection,
`MARREQ_RECOVERY_ROOT` to the private directory containing `bundle/`, and maintain
quiescence. The bundle must be readable by UID 10001. Select the correct base file:

```sh
docker compose -f docker/docker-compose.light.yml \
  -f docker/docker-compose.migrate.yml run --rm migrate
# Only after a zero exit status: deploy selected images, then real authenticated smoke.
```

The one-shot job exposes no ports/socket, drops capabilities, uses a read-only
root/recovery mount, never seeds and never restores. It waits for Compose database
health; native commands fail promptly on connection errors. The job validates
backup identity/history/references and restore receipt while holding the migration
lock. A no-pending rerun validates history, core projections and the legacy bootstrap credential gate without a new backup.
Persistent DB volumes are unchanged.

Future #423 orchestration must gate each next step on zero exit: candidate/API
compatibility → stop writers → coherent backup → isolated restore and retained
smoke → serialized migration → schema check → candidate and retained HTTP smoke
for an overlapping window → selected deployment → cross-service smoke → deployment
record. Migration-history checks alone do not prove arbitrary schema semantics or
application health. No production CD, global version registry, automatic database
rollback or image publication is introduced here.

## Reproducible validation

```sh
python3 scripts/check-migration-policy.py
scripts/test-migrations.sh
scripts/test-migration-image.sh marreq-server:local
scripts/test-migration-image.sh marreq-cloud:local
scripts/test-api-contracts.sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

The migration suite creates disposable PostgreSQL containers/databases, exercises
fresh/repeated/multiple migrations, two concurrent jobs, contention timeout,
transaction failure/retry, process interruption, invalid history/permissions,
expansion compatibility, real SQL/blob backup and restore, corrupt/missing files,
and production recovery gates. Backend image CI checks startup does not mutate an
unprepared DB and exercises explicit migration plus expanded-schema HTTP startup
for both variants. The existing contract suite supplies representative real
frontend/MCP/Server/Cloud exchanges; release-specific retained-image compatibility
still requires actual release artifacts and is not inferred from fixtures.
