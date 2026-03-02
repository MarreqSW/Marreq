#!/bin/sh
set -e

if [ -n "${DATABASE_URL}" ]; then
  echo "Waiting for database..."
  until psql "${DATABASE_URL}" -c "SELECT 1" >/dev/null 2>&1; do
    echo "Database is unavailable - sleeping"
    sleep 2
  done
  echo "Database is up."

  echo "Running migrations..."
  diesel migration run
  echo "Migrations complete."

  AUTO_SEED="${REQMAN_AUTO_SEED:-true}"
  if [ "$AUTO_SEED" = "true" ]; then
    HAS_PROJECTS_DATA=$(psql "${DATABASE_URL}" -t -A -c "SELECT EXISTS (SELECT 1 FROM projects LIMIT 1);" 2>/dev/null | tr -d '[:space:]' || echo "f")
    if [ "$HAS_PROJECTS_DATA" != "t" ]; then
      echo "No projects found. Running sample data seed (init_complete.sql)..."
      psql "${DATABASE_URL}" -f /app/scripts/init_complete.sql
      echo "Sample data seed complete."
    else
      echo "Projects already exist; skipping sample data seed."
    fi
  fi
fi

exec "$@"
