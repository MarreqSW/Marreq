#!/bin/bash
set -euo pipefail

echo "=========================================="
echo "ReqMan Database Cleanup Script"
echo "=========================================="
echo ""

# Pick compose command (v2 'docker compose' or legacy 'docker-compose')
if docker compose version >/dev/null 2>&1; then
  DC="docker compose"
elif docker-compose version >/dev/null 2>&1; then
  DC="docker-compose"
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

echo "💣 Dropping database 'reqman' if it exists..."
psqlc -U rust -d postgres -c "DROP DATABASE IF EXISTS reqman WITH (FORCE);"
echo "✅ Database 'reqman' dropped"
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

echo "📋 Verifying that 'reqman' database no longer exists..."
if psqlc -U rust -d postgres -c "SELECT 1 FROM pg_database WHERE datname='reqman';" | grep -qx "1"; then
  echo "❌ Error: 'reqman' still exists!"
  exit 1
fi
echo "✅ 'reqman' successfully removed"
echo ""

echo "=========================================="
echo "🎉 ReqMan Database Cleanup Complete!"
echo "=========================================="
echo ""
