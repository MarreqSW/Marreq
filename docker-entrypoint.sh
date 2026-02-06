#!/bin/sh
set -e

# Wait for PostgreSQL and run migrations (idempotent)
if [ -n "${DATABASE_URL}" ]; then
  echo "Waiting for database..."
  until diesel migration run; do
    echo "Database is unavailable - sleeping"
    sleep 2
  done
  echo "Migrations complete."
fi

exec "$@"
