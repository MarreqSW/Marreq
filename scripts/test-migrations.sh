#!/usr/bin/env bash
# Isolated PostgreSQL only: deliberately ignores developer DATABASE_URL and .env.
set -euo pipefail
cd "$(dirname "$0")/.."
work=$(mktemp -d)
container="marreq-migrations-$RANDOM-$$"
app_pid=
cleanup() { if [[ -n "$app_pid" ]]; then kill "$app_pid" 2>/dev/null || true; wait "$app_pid" 2>/dev/null || true; fi; docker rm -f "$container" >/dev/null 2>&1 || true; rm -rf "$work"; }
trap cleanup EXIT
image=pgvector/pgvector:pg17@sha256:ac08538c6f8b9904c33c8224c5e5706dbe760aca29db1d096972b4052c22a75d
docker run --detach --rm --name "$container" --tmpfs /var/lib/postgresql/data \
  -e POSTGRES_PASSWORD=disposable -p 127.0.0.1::5432 "$image" >/dev/null
for attempt in {1..60}; do
  if docker exec "$container" pg_isready -U postgres >/dev/null 2>&1; then break; fi
  sleep 1
done
docker exec "$container" pg_isready -U postgres >/dev/null
port=$(docker port "$container" 5432/tcp); port=${port##*:}
export MARREQ_MIGRATION_TEST_URL="postgres://postgres:disposable@127.0.0.1:$port/postgres"
export DATABASE_URL="$MARREQ_MIGRATION_TEST_URL"
unset MARREQ_SCHEMA_COMPATIBLE_THROUGH
cargo test --locked -p marreq-migrations -- --include-ignored --test-threads=1
cargo build --locked -p marreq-migrations -p marreq-core -p marreq-server -p marreq-cloud --bin marreq-migrations --bin marreq-backup --bin marreq-server --bin marreq-cloud
binary_root="$PWD/target/debug"
repo_root="$PWD"
runner="$PWD/target/debug/marreq-migrations"
backup="$PWD/target/debug/marreq-backup"
# Two independent CLI processes race to initialize the same empty DB.
docker exec "$container" createdb -U postgres concurrent_cli
DATABASE_URL="postgres://postgres:disposable@127.0.0.1:$port/concurrent_cli" "$runner" dev-init > "$work/concurrent-a.log" 2>&1 &
first=$!
DATABASE_URL="postgres://postgres:disposable@127.0.0.1:$port/concurrent_cli" "$runner" dev-init > "$work/concurrent-b.log" 2>&1 &
second=$!
wait "$first"; wait "$second"
DATABASE_URL="postgres://postgres:disposable@127.0.0.1:$port/concurrent_cli" "$runner" check
"$runner" dev-init
# Real content-addressed blob and database reference.
export MARREQ_ATTACHMENTS_DIR="$work/blobs" TMPDIR="$work"
blob=$(printf 'migration recovery fixture' | sha256sum); blob=${blob%% *}
mkdir -p "$MARREQ_ATTACHMENTS_DIR/${blob:0:2}/${blob:2:2}"
printf 'migration recovery fixture' > "$MARREQ_ATTACHMENTS_DIR/${blob:0:2}/${blob:2:2}/$blob"
docker exec -i "$container" psql -v ON_ERROR_STOP=1 -U postgres <<SQL
INSERT INTO projects(name,slug,owner_id) VALUES ('Recovery fixture','recovery-fixture',(SELECT min(id) FROM users));
INSERT INTO attachments(project_id,entity_type,entity_id,sha256,size_bytes,original_filename,content_type)
VALUES ((SELECT max(id) FROM projects),'requirement',1,'$blob',26,'fixture.txt','text/plain');
SQL
# Use the matching PG17 client without installing host packages.
cat > "$work/pg_dump" <<'WRAPPER'
#!/usr/bin/env bash
exec docker run --rm --network host --user "$(id -u):$(id -g)" \
  -e PGPASSWORD -v "$TMPDIR:$TMPDIR" \
  pgvector/pgvector:pg17@sha256:ac08538c6f8b9904c33c8224c5e5706dbe760aca29db1d096972b4052c22a75d pg_dump "$@"
WRAPPER
chmod +x "$work/pg_dump"
export MARREQ_PG_DUMP="$work/pg_dump" MARREQ_RECOVERY_BUNDLE="$work/bundle"
if "$backup"; then echo 'backup accepted missing quiescence' >&2; exit 1; fi
export MARREQ_WRITES_QUIESCED=true
"$backup"
"$runner" verify-backup
# libpq URI encoding must survive credential removal in a real pg_dump connection.
# This role and all credentials exist only in the disposable test container.
docker exec "$container" psql -U postgres -v ON_ERROR_STOP=1 -c "CREATE ROLE backup_uri LOGIN SUPERUSER PASSWORD 's e+c+ret=end'" > /dev/null
for credential_location in authority query; do
  if [[ "$credential_location" == authority ]]; then
    encoded_url="postgres://backup_uri:s%20e%2Bc+ret%3Dend@127.0.0.1:$port/postgres?options=-c%20statement_timeout%3D10000&application_name=backup%2Buri+literal"
  else
    encoded_url="postgres://backup_uri@127.0.0.1:$port/postgres?options=-c%20statement_timeout%3D10000&password=s%20e%2Bc+ret%3Dend&application_name=backup%2Buri+literal"
  fi
  DATABASE_URL="$encoded_url" MARREQ_RECOVERY_BUNDLE="$work/uri-$credential_location" "$backup"
  DATABASE_URL="$encoded_url" MARREQ_RECOVERY_BUNDLE="$work/uri-$credential_location" "$runner" verify-backup
done
docker exec "$container" psql -U postgres -v ON_ERROR_STOP=1 -c 'DROP ROLE backup_uri' > /dev/null
# Corruption and missing-artifact checks.
cp "$work/bundle/backup.tar.gz" "$work/original.tar.gz"
printf corrupt >> "$work/bundle/backup.tar.gz"
if "$runner" verify-backup; then echo 'corrupt backup accepted' >&2; exit 1; fi
mv "$work/bundle/backup.tar.gz" "$work/corrupt.tar.gz"
if "$runner" verify-backup; then echo 'missing backup accepted' >&2; exit 1; fi
cp "$work/original.tar.gz" "$work/bundle/backup.tar.gz"
# Restore actual SQL and attachments to a new, disposable database/directory.
docker exec "$container" createdb -U postgres restored
mkdir "$work/restored"
tar -xzf "$work/bundle/backup.tar.gz" -C "$work/restored"
restored_root=$(find "$work/restored" -mindepth 1 -maxdepth 1 -type d)
docker exec -i "$container" psql -X -v ON_ERROR_STOP=1 --single-transaction -U postgres -d restored < "$restored_root/database.sql" > "$work/restore.log"
export DATABASE_URL="postgres://postgres:disposable@127.0.0.1:$port/restored"
export MARREQ_ATTACHMENTS_DIR="$restored_root/attachments"
"$runner" check
# Run both real backend binaries on the recovered database and recovered files.
for variant in server cloud; do
  app_port=$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1]); s.close()')
  app_url="http://127.0.0.1:$app_port"
  (cd "$work" && exec env -i PATH="$PATH" LD_LIBRARY_PATH="${LD_LIBRARY_PATH:-}" \
    DATABASE_URL="$DATABASE_URL" ROCKET_DATABASES="{my_db={url=\"$DATABASE_URL\",pool_size=2}}" \
    ROCKET_ADDRESS=127.0.0.1 ROCKET_PORT="$app_port" \
    MARREQ_PUBLIC_BASE_URL="$app_url" MARREQ_MCP_PUBLIC_URL="$app_url/mcp" \
    MARREQ_MCP_AUDIT_SECRET=disposable-recovery-smoke-audit-secret \
    MARREQ_ATTACHMENTS_DIR="$MARREQ_ATTACHMENTS_DIR" CSRF_ALLOWED_ORIGINS="$app_url" \
    MARREQ_UI_MODE=api_only "$binary_root/marreq-$variant") > "$work/$variant.log" 2>&1 &
  app_pid=$!
  ready=false
  for attempt in {1..60}; do
    if ! kill -0 "$app_pid" 2>/dev/null; then cat "$work/$variant.log"; exit 1; fi
    if curl -fsS "$app_url/api/auth/csrf" > /dev/null 2>&1; then ready=true; break; fi
    sleep 1
  done
  if [[ "$ready" != true ]]; then cat "$work/$variant.log"; exit 1; fi
  python3 "$repo_root/scripts/smoke-restored-backend.py" "$app_url"
  kill "$app_pid"; wait "$app_pid" || true; app_pid=
done
"$runner" verify-restored
# Lost recovered blob must be detected, even with a valid SQL restore.
rm "$MARREQ_ATTACHMENTS_DIR/${blob:0:2}/${blob:2:2}/$blob"
if "$runner" verify-restored; then echo 'missing restored blob accepted' >&2; exit 1; fi
"$runner" dev-init # no pending migrations; the restored fixture deliberately retains its demo credential
# Production pending migration gate cannot be bypassed by omitting recovery evidence.
docker exec "$container" createdb -U postgres unprepared
export DATABASE_URL="postgres://postgres:disposable@127.0.0.1:$port/unprepared"
unset MARREQ_RECOVERY_BUNDLE
if "$runner" apply; then echo 'production migration accepted missing backup' >&2; exit 1; fi
# Exercise the successful production gate too, including a real restore of this exact bundle.
export MARREQ_RECOVERY_BUNDLE="$work/fresh-bundle" MARREQ_ATTACHMENTS_DIR="$work/empty-blobs"
mkdir "$MARREQ_ATTACHMENTS_DIR"
"$backup"
if "$runner" apply; then echo 'production accepted missing restore receipt' >&2; exit 1; fi
docker exec "$container" createdb -U postgres fresh_rehearsal
mkdir "$work/fresh-restored"
tar -xzf "$work/fresh-bundle/backup.tar.gz" -C "$work/fresh-restored"
fresh_root=$(find "$work/fresh-restored" -mindepth 1 -maxdepth 1 -type d)
docker exec -i "$container" psql -X -v ON_ERROR_STOP=1 --single-transaction -U postgres -d fresh_rehearsal < "$fresh_root/database.sql" > "$work/fresh-restore.log"
DATABASE_URL="postgres://postgres:disposable@127.0.0.1:$port/fresh_rehearsal" "$runner" verify-restored
if "$runner" apply; then echo 'legacy public bootstrap credential accepted' >&2; exit 1; fi
# Explicit operator remediation in this disposable fresh database (no silent data deletion).
docker exec "$container" psql -U postgres -d unprepared -v ON_ERROR_STOP=1 -c "UPDATE users SET password_hash=NULL WHERE username='alice'" > /dev/null
"$runner" apply # no pending work: validates schema/bootstrap without demanding another backup
"$runner" check
printf 'Migration concurrency, failure, compatibility and recovery tests passed\n' 
