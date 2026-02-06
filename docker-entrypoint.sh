#!/bin/sh
set -e

if [ -n "${DATABASE_URL}" ]; then
  echo "Waiting for database..."
  until psql "${DATABASE_URL}" -c "SELECT 1" >/dev/null 2>&1; do
    echo "Database is unavailable - sleeping"
    sleep 2
  done
  echo "Database is up."

  # If schema does not exist, run init_complete.sql (schema + seed) then mark migrations applied.
  # This avoids races with initdb.d and ensures diesel does not re-run baseline.
  HAS_PROJECTS=$(psql "${DATABASE_URL}" -t -A -c "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema='public' AND table_name='projects');" 2>/dev/null | tr -d '[:space:]' || echo "f")
  if [ "$HAS_PROJECTS" != "t" ]; then
    echo "Schema not found. Running init_complete.sql..."
    psql "${DATABASE_URL}" -f /app/scripts/init_complete.sql
    echo "Schema and seed data initialized."
  fi

  # Mark baseline/seed as applied when schema exists (so diesel skips them).
  psql "${DATABASE_URL}" -f /app/scripts/backfill_diesel_migrations.sql

  if [ "$HAS_PROJECTS" != "t" ]; then
    echo "Running migrations..."
    diesel migration run
    echo "Migrations complete."
  else
    echo "Schema already present; migrations marked applied."
  fi
fi

exec "$@"
