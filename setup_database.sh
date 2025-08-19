#!/bin/bash
set -euo pipefail

echo "=========================================="
echo "ReqMan Database Setup Script"
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

# Make sure the init file exists (so the redirect won't silently pass an empty stream)
INIT_SQL="${INIT_SQL:-init_complete.sql}"
if [[ ! -f "${INIT_SQL}" ]]; then
  echo "❌ Error: '${INIT_SQL}' not found in $(pwd)."
  echo "   Set INIT_SQL=/path/to/file.sql or place init_complete.sql here."
  exit 1
fi

# Helper to run psql inside the container
psqlc() {
  docker exec -i "${DB_CID}" psql -v ON_ERROR_STOP=1 -qAt "$@"
}

echo "📊 Creating database 'reqman' if it doesn't exist..."
# returns '1' if exists, nothing otherwise
if ! psqlc -U rust -d postgres -c "SELECT 1 FROM pg_database WHERE datname='reqman';" | grep -qx "1"; then
  psqlc -U rust -d postgres -c "CREATE DATABASE reqman;"
fi
echo "✅ Database 'reqman' is ready"
echo ""

echo "🧹 Cleaning existing tables (if any)..."
psqlc -U rust -d reqman <<'SQL' >/dev/null 2>&1 || true
DROP TABLE IF EXISTS matrix CASCADE;
DROP TABLE IF EXISTS logs CASCADE;
DROP TABLE IF EXISTS requirements CASCADE;
DROP TABLE IF EXISTS tests CASCADE;
DROP TABLE IF EXISTS users CASCADE;
DROP TABLE IF EXISTS categories CASCADE;
DROP TABLE IF EXISTS applicability CASCADE;
DROP TABLE IF EXISTS verification CASCADE;
DROP TABLE IF EXISTS status CASCADE;
DROP TABLE IF EXISTS projects CASCADE;
SQL
echo "✅ Database cleaned"
echo ""

echo "🚀 Initializing database with complete schema and data..."
# Feed the SQL file into psql inside the container
docker exec -i "${DB_CID}" psql -v ON_ERROR_STOP=1 -U rust -d reqman < "${INIT_SQL}"
echo "✅ Database initialization completed successfully!"
echo ""

echo "🔍 Verifying database setup..."
echo "📋 Tables created:"
docker exec -i "${DB_CID}" psql -U rust -d reqman -c "\dt" \
  | grep -E "(projects|users|requirements|tests|matrix|logs|categories|applicability|verification|status)" || true
echo ""

echo "👥 Users created:"
docker exec -i "${DB_CID}" psql -U rust -d reqman -c \
  "SELECT user_username, user_name, is_admin FROM users ORDER BY user_id;"
echo ""

echo "📁 Projects created:"
docker exec -i "${DB_CID}" psql -U rust -d reqman -c \
  "SELECT project_id, project_name, project_status FROM projects ORDER BY project_id;"
echo ""

echo "📊 Sample data counts:"
docker exec -i "${DB_CID}" psql -U rust -d reqman -c "
SELECT 'Requirements' AS entity, COUNT(*) FROM requirements
UNION ALL
SELECT 'Tests', COUNT(*) FROM tests
UNION ALL
SELECT 'Matrix Links', COUNT(*) FROM matrix
UNION ALL
SELECT 'Categories', COUNT(*) FROM categories
UNION ALL
SELECT 'Applicability', COUNT(*) FROM applicability
UNION ALL
SELECT 'Logs', COUNT(*) FROM logs;
"
echo ""
echo "=========================================="
echo "🎉 ReqMan Database Setup Complete!"
echo "=========================================="
echo ""
echo "📝 Login Credentials (all users have password: 'password'):"
echo "   • alice (Admin) - Alice Johnson"
echo "   • dr_smith (Admin) - Dr. Sarah Smith"
echo "   • eng_jones - Engineer Mike Jones"
echo "   • tech_lee - Technician Lisa Lee"
echo "   • qa_wilson - QA Specialist Tom Wilson"
echo "   • admin (Admin) - System Administrator"
echo ""
echo "🌐 Application URL: http://localhost:8000"
echo ""
echo "🚀 To start the application:"
echo "   cargo run --bin req_man"
echo ""
echo "=========================================="
