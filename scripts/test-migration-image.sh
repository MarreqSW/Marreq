#!/usr/bin/env bash
# Test either real backend image against a disposable database. Never loads .env.
set -euo pipefail
image=${1:?backend image required}
name="marreq-migration-image-$RANDOM-$$"
work=$(mktemp -d)
cleanup() { docker rm -f "$name-app" "$name-db" >/dev/null 2>&1 || true; docker network rm "$name" >/dev/null 2>&1 || true; rm -rf "$work"; }
trap cleanup EXIT
docker network create "$name" >/dev/null
docker run -d --rm --name "$name-db" --network "$name" --network-alias db --tmpfs /var/lib/postgresql/data \
  -e POSTGRES_PASSWORD=disposable pgvector/pgvector:pg17@sha256:ac08538c6f8b9904c33c8224c5e5706dbe760aca29db1d096972b4052c22a75d >/dev/null
for attempt in {1..60}; do if docker exec "$name-db" pg_isready -U postgres >/dev/null 2>&1; then break; fi; sleep 1; done
url=postgres://postgres:disposable@db/postgres
env_args=(-e "DATABASE_URL=$url" -e "ROCKET_DATABASES={my_db={url=\"$url\"}}" -e ROCKET_ADDRESS=0.0.0.0 -e ROCKET_PORT=8000 -e MARREQ_AUTO_SEED=false -e MARREQ_UI_MODE=api_only \
  -e MARREQ_PUBLIC_BASE_URL=http://localhost:8000 -e MARREQ_MCP_PUBLIC_URL=http://localhost:8000/mcp \
  -e MARREQ_MCP_AUDIT_SECRET=disposable-integration-audit-secret-32 \
  -e ROCKET_SECRET_KEY=UT5jxByH2kV2h9j/620KVP2w5NwZUEGVREFBbIKHVZk=)
if docker run --rm --network "$name" "${env_args[@]}" "$image" > "$work/unprepared.log" 2>&1; then echo 'backend started against an unprepared DB' >&2; exit 1; fi
# No startup-side mutation, including Diesel's tracking table.
test "$(docker exec "$name-db" psql -U postgres -tAc "SELECT count(*) FROM information_schema.tables WHERE table_schema='public'")" = 0
if docker run --rm --network "$name" "${env_args[@]}" --entrypoint marreq-migrations "$image" apply; then echo 'production apply accepted missing recovery' >&2; exit 1; fi
docker run --rm --network "$name" "${env_args[@]}" --entrypoint marreq-migrations "$image" dev-init
# Additive schema fixture: retained SQL must remain operational; unknown history needs approval.
docker exec "$name-db" psql -U postgres -v ON_ERROR_STOP=1 -c "ALTER TABLE projects ADD COLUMN expansion_fixture text; INSERT INTO __diesel_schema_migrations(version) VALUES ('20990101000001')" >/dev/null
if docker run --rm --network "$name" "${env_args[@]}" "$image" > "$work/unreviewed.log" 2>&1; then echo 'unreviewed schema accepted' >&2; exit 1; fi
docker run -d --rm --name "$name-app" --network "$name" "${env_args[@]}" -e MARREQ_SCHEMA_COMPATIBLE_THROUGH=20990101000001 "$image" >/dev/null
ready=false
for attempt in {1..60}; do
  if docker exec "$name-app" curl -fsS http://localhost:8000/api/auth/csrf > "$work/csrf.json" 2>/dev/null; then ready=true; break; fi
  sleep 1
done
if [[ "$ready" != true ]]; then docker logs "$name-app"; exit 1; fi
docker exec "$name-app" curl -fsS http://localhost:8000/api/meta/deployment > "$work/deployment.json"
# Authenticated database-backed smoke on the expanded schema using the historical fixture user.
docker exec "$name-app" curl -fsS -c /tmp/migration-cookies http://localhost:8000/api/auth/csrf > "$work/csrf.json"
csrf=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["csrf_token"])' "$work/csrf.json")
docker exec "$name-app" curl -fsS -b /tmp/migration-cookies -c /tmp/migration-cookies \
  -H 'Content-Type: application/json' -H "X-CSRF-Token: $csrf" \
  -H 'Origin: http://localhost:8000' \
  -d '{"username":"alice","password":"ChangeMe123!"}' http://localhost:8000/api/auth/login > "$work/login.json"
docker exec "$name-app" curl -fsS -b /tmp/migration-cookies http://localhost:8000/api/auth/me > "$work/me.json"
python3 -c 'import json,sys; assert json.load(open(sys.argv[1]))["username"] == "alice"' "$work/me.json"

docker exec "$name-app" sh -ec 'test "$(id -u)" != 0; ! command -v diesel; test ! -e /app/tools'
if [[ $# == 4 ]]; then
  python3 scripts/test-artifact-image.py "$2" "$image" "$3" "$4" --running-container "$name-app"
fi
printf 'Backend image startup, explicit migrations and expanded-schema HTTP smoke passed: %s\n' "$image"
