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
  echo "⚠️  The 'db' service isn't running. Attempting to start it..."
  if ! $DC up -d db >/dev/null; then
    echo "❌ Error: Failed to start the 'db' service using '$DC up -d db'."
    echo "   Please start it manually and re-run this script."
    exit 1
  fi
  # Give Docker a brief moment to report the container as running
  sleep 2
  DB_CID=$($DC ps -q db || true)
fi

if [[ -z "${DB_CID}" ]]; then
  echo "❌ Error: The 'db' service still isn't running after attempting to start it."
  echo "   Check 'docker compose logs db' for details."
  exit 1
fi
echo "✅ Database container is running: ${DB_CID}"
echo ""

echo "⏳ Waiting for PostgreSQL to be ready..."
MAX_RETRIES=30
RETRY_COUNT=0
until docker exec "${DB_CID}" pg_isready -U rust -q 2>/dev/null; do
  RETRY_COUNT=$((RETRY_COUNT + 1))
  if [[ ${RETRY_COUNT} -ge ${MAX_RETRIES} ]]; then
    echo "❌ Error: PostgreSQL failed to become ready after ${MAX_RETRIES} attempts."
    echo "   Container logs:"
    docker logs "${DB_CID}" --tail 20
    exit 1
  fi
  # Check if container is still running
  if ! docker ps -q --filter "id=${DB_CID}" | grep -q .; then
    echo "❌ Error: Database container stopped unexpectedly."
    echo "   Container logs:"
    docker logs "${DB_CID}" --tail 20
    exit 1
  fi
  echo "   Waiting for database... (attempt ${RETRY_COUNT}/${MAX_RETRIES})"
  sleep 2
done
echo "✅ PostgreSQL is ready to accept connections"
echo ""

# Determine the script's directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "${SCRIPT_DIR}")"

# Make sure the init file exists (so the redirect won't silently pass an empty stream)
# Try multiple locations in order of preference
if [[ -z "${INIT_SQL:-}" ]]; then
  if [[ -f "${SCRIPT_DIR}/init_complete.sql" ]]; then
    INIT_SQL="${SCRIPT_DIR}/init_complete.sql"
  elif [[ -f "${PROJECT_ROOT}/init_complete.sql" ]]; then
    INIT_SQL="${PROJECT_ROOT}/init_complete.sql"
  elif [[ -f "${PROJECT_ROOT}/sql/init_complete.sql" ]]; then
    INIT_SQL="${PROJECT_ROOT}/sql/init_complete.sql"
  else
    INIT_SQL="init_complete.sql"
  fi
fi

if [[ ! -f "${INIT_SQL}" ]]; then
  echo "❌ Error: '${INIT_SQL}' not found."
  echo "   Searched in:"
  echo "   - ${SCRIPT_DIR}/init_complete.sql"
  echo "   - ${PROJECT_ROOT}/init_complete.sql"
  echo "   - ${PROJECT_ROOT}/sql/init_complete.sql"
  echo "   Set INIT_SQL=/path/to/file.sql or place init_complete.sql in one of the above locations."
  exit 1
fi

echo "📄 Using SQL file: ${INIT_SQL}"
echo ""

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
DROP TABLE IF EXISTS project_members CASCADE;
DROP TABLE IF EXISTS users CASCADE;
DROP TABLE IF EXISTS categories CASCADE;
DROP TABLE IF EXISTS applicability CASCADE;
DROP TABLE IF EXISTS verification CASCADE;
DROP TABLE IF EXISTS requirement_status CASCADE;
DROP TABLE IF EXISTS status_id CASCADE;
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
  | grep -E "(projects|users|requirements|tests|matrix|logs|categories|applicability|verification|requirement_status|status_id)" || true
echo ""

echo "👥 Users created:"
docker exec -i "${DB_CID}" psql -U rust -d reqman -c \
  "SELECT username, name, is_admin FROM users ORDER BY id;"
echo ""

echo "📁 Projects created:"
docker exec -i "${DB_CID}" psql -U rust -d reqman -c \
  "SELECT id, name, status FROM projects ORDER BY id;"
echo ""

echo "🤝 Project memberships:"
docker exec -i "${DB_CID}" psql -U rust -d reqman <<'SQL'
SELECT pm.project_id,
       p.name,
       pm.user_id,
       u.username,
       pm.role,
       CASE pm.role
           WHEN 1 THEN 'Owner'
           WHEN 2 THEN 'Manager'
           WHEN 3 THEN 'Contributor'
           WHEN 4 THEN 'Viewer'
           ELSE 'Member'
       END AS role_name
  FROM project_members pm
  JOIN projects p ON p.id = pm.project_id
  JOIN users u ON u.id = pm.user_id
 ORDER BY pm.project_id, pm.user_id;
SQL
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
SELECT 'Requirement Status', COUNT(*) FROM requirement_status
UNION ALL
SELECT 'Test Status', COUNT(*) FROM test_status
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
