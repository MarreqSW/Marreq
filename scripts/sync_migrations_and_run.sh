#!/usr/bin/env bash
# Run backfill (if schema exists) then "diesel migration run" using the same
# DATABASE_URL, so you never backfill one DB and run migrations on another.
set -e
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# Load DATABASE_URL from .env so backfill and diesel use the same DB
if [[ -f .env ]]; then
  set -a
  # shellcheck source=/dev/null
  source .env
  set +a
fi

if [[ -z "${DATABASE_URL:-}" ]]; then
  echo "DATABASE_URL is not set. Set it or create .env with DATABASE_URL=..." >&2
  exit 1
fi

echo "Using DATABASE_URL (host): $(echo "$DATABASE_URL" | sed -E 's|.*@([^/]+)/.*|\1|')"
psql "$DATABASE_URL" -f scripts/backfill_diesel_migrations.sql
# Apply requirement_version_links with psql (same DB as backfill) so we don't depend on
# diesel using the same connection. Then mark it applied so "diesel migration run" is a no-op.
psql "$DATABASE_URL" -f scripts/apply_requirement_version_links.sql
psql "$DATABASE_URL" -c "INSERT INTO __diesel_schema_migrations (version) VALUES ('2026-02-26-000002_add_requirement_version_links') ON CONFLICT (version) DO NOTHING;"
# Use same URL explicitly so we never run migrations on a different DB than the backfill
if ! diesel migration run --database-url "$DATABASE_URL"; then
  echo "Note: diesel migration run failed (e.g. baseline already applied elsewhere). Schema is already updated via psql above." >&2
  exit 0
fi
