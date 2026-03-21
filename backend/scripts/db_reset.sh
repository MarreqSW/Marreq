#!/bin/bash
set -euo pipefail

# db_reset.sh — Drop the Marreq database entirely
#
# ⚠  DESTRUCTIVE — permanently deletes all data.  Use for development resets.
#
# Typical workflow after this script:
#   ./backend/scripts/db_setup.sh        # re-apply schema via Diesel
#   ./backend/scripts/db_seed.sh         # optionally reload demo/test data
#
# Prerequisites:
#   • Docker running with the 'db' service up.

echo "=========================================="
echo "Marreq — Database Reset (DROP DATABASE)"
echo "=========================================="
echo ""

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BACKEND_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
REPO_ROOT="$(cd "${BACKEND_ROOT}/.." && pwd)"
COMPOSE_FILE="${REPO_ROOT}/docker/docker-compose.yml"

# Pick compose command (v2 'docker compose' or legacy 'docker-compose')
if docker compose version >/dev/null 2>&1; then
  DC="docker compose -f ${COMPOSE_FILE}"
elif docker-compose version >/dev/null 2>&1; then
  DC="docker-compose -f ${COMPOSE_FILE}"
else
  echo "❌ Error: Neither 'docker compose' nor 'docker-compose' found."
  exit 1
fi

# Check Docker daemon
if ! docker info >/dev/null 2>&1; then
  echo "❌ Error: Docker is not running. Please start Docker first."
  exit 1
fi

# Ensure stack is up
echo "🔎 Checking for running 'db' service..."
DB_CID=$($DC ps -q db || true)
if [[ -z "${DB_CID}" ]]; then
  echo "❌ Error: The 'db' service isn't running."
  echo "   Start it with: $DC up -d"
  exit 1
fi
echo "✅ Database container is running: ${DB_CID}"
echo ""

# Helper to run psql inside the container
psqlc() {
  docker exec -i "${DB_CID}" psql -v ON_ERROR_STOP=1 -qAt "$@"
}

echo "💣 Dropping database 'marreq' if it exists..."
psqlc -U rust -d postgres -c "DROP DATABASE IF EXISTS marreq WITH (FORCE);"
echo "✅ Database 'marreq' dropped"
echo ""

echo "🧹 Dropping any leftover objects owned by user 'rust'..."
psqlc -U rust -d postgres <<'SQL' || true
DO
$$
DECLARE
    r RECORD;
BEGIN
    FOR r IN SELECT 'DROP OWNED BY rust CASCADE;' AS stmt LOOP
        EXECUTE r.stmt;
    END LOOP;
END
$$;
SQL
echo "✅ User-owned objects cleaned"
echo ""

echo "📋 Verifying that 'marreq' database no longer exists..."
if psqlc -U rust -d postgres -c "SELECT 1 FROM pg_database WHERE datname='marreq';" | grep -qx "1"; then
  echo "❌ Error: 'marreq' still exists!"
  exit 1
fi
echo "✅ 'marreq' successfully removed"
echo ""

echo "=========================================="
echo "🎉 Marreq Database Cleanup Complete!"
echo "=========================================="
echo ""
