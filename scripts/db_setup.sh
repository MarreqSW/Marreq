#!/bin/bash
set -euo pipefail

# db_setup.sh — Fresh database setup for Marreq
#
# Creates the marreq database (if absent) and applies all pending migrations
# using Diesel.  Diesel is the single authority for schema management; no
# manual DDL is executed by this script.
#
# Usage:
#   ./scripts/db_setup.sh           # apply schema only
#   ./scripts/db_setup.sh --seed    # apply schema, then load demo/test data
#
# Prerequisites:
#   • Docker running with Compose (for the managed PostgreSQL container), OR
#     a PostgreSQL server already accessible at DATABASE_URL.
#   • diesel CLI installed:
#       cargo install diesel_cli --no-default-features --features postgres
#   • DATABASE_URL in .env or environment
#       default: postgres://rust:rust@127.0.0.1:5432/marreq
#
# For non-Docker (bare-metal) setups, ensure DATABASE_URL points to your
# server and that psql is available in PATH.  The script auto-detects whether
# Docker is in use.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "${SCRIPT_DIR}")"
SEED=false

for arg in "$@"; do
  case $arg in
    --seed) SEED=true ;;
    *) echo "Unknown argument: $arg" >&2; exit 1 ;;
  esac
done

# ── Colors ───────────────────────────────────────────────────────────────────
GREEN='\033[0;32m'; YELLOW='\033[1;33m'
RED='\033[0;31m';   BLUE='\033[0;34m'; NC='\033[0m'
info()    { echo -e "${BLUE}ℹ  $*${NC}"; }
success() { echo -e "${GREEN}✅ $*${NC}"; }
warn()    { echo -e "${YELLOW}⚠  $*${NC}"; }
error()   { echo -e "${RED}❌ $*${NC}" >&2; exit 1; }

echo -e "${BLUE}"
echo "==========================================="
echo "   Marreq — Database Setup"
echo "==========================================="
echo -e "${NC}"

# ── Load .env ────────────────────────────────────────────────────────────────
if [[ -f "${PROJECT_ROOT}/.env" ]]; then
  info "Loading ${PROJECT_ROOT}/.env"
  set -a; source "${PROJECT_ROOT}/.env"; set +a
fi

# Default credentials — must match docker-compose.yml
DATABASE_URL="${DATABASE_URL:-postgres://rust:rust@127.0.0.1:5432/marreq}"
DB_USER="${DATABASE_URL%@*}"           # strip host/port/db
DB_USER="${DB_USER##*:}"              # strip scheme and name, keep password
DB_USER="${DATABASE_URL#*://}"        # scheme://user:pass@...
DB_USER="${DB_USER%%:*}"              # keep only the user portion
# Parse db name (last path component, before any query string)
DB_NAME="${DATABASE_URL##*/}"
DB_NAME="${DB_NAME%%\?*}"

# ── Require diesel CLI ───────────────────────────────────────────────────────
if ! command -v diesel &>/dev/null; then
  error "diesel CLI not found.
  Install it with:
    cargo install diesel_cli --no-default-features --features postgres"
fi

# ── Detect Docker Compose ────────────────────────────────────────────────────
USE_DOCKER=false
DC=""
if docker compose version >/dev/null 2>&1; then
  DC="docker compose"; USE_DOCKER=true
elif docker-compose version >/dev/null 2>&1; then
  DC="docker-compose"; USE_DOCKER=true
fi

# ── Start db container (Docker path) ────────────────────────────────────────
DB_CID=""
if [[ "${USE_DOCKER}" == "true" ]]; then
  if ! docker info >/dev/null 2>&1; then
    error "Docker daemon is not running. Start Docker and retry, or set USE_DOCKER=false to target a bare-metal server."
  fi
  info "Starting PostgreSQL container (db service)..."
  cd "${PROJECT_ROOT}"
  ${DC} up -d db
  DB_CID=$(${DC} ps -q db || true)
  if [[ -z "${DB_CID}" ]]; then
    error "Could not determine container ID for the 'db' service."
  fi
fi

# ── Wait for PostgreSQL ──────────────────────────────────────────────────────
info "Waiting for PostgreSQL to be ready..."
MAX_RETRIES=30
for ((i=1; i<=MAX_RETRIES; i++)); do
  if [[ "${USE_DOCKER}" == "true" ]]; then
    docker exec "${DB_CID}" pg_isready -q 2>/dev/null && break
  else
    pg_isready -d "${DATABASE_URL}" -q 2>/dev/null && break
  fi
  [[ $i -ge ${MAX_RETRIES} ]] && error "PostgreSQL not ready after ${MAX_RETRIES} attempts. Check that the server is running."
  echo "   attempt ${i}/${MAX_RETRIES}..."
  sleep 2
done
success "PostgreSQL is ready"

# ── Create database if absent ─────────────────────────────────────────────────
echo ""
info "Checking database '${DB_NAME}'..."
if [[ "${USE_DOCKER}" == "true" ]]; then
  EXISTS=$(docker exec "${DB_CID}" psql -U "${DB_USER}" -d postgres -tAc \
    "SELECT 1 FROM pg_database WHERE datname='${DB_NAME}'" || true)
  if [[ "${EXISTS}" != "1" ]]; then
    docker exec "${DB_CID}" psql -U "${DB_USER}" -d postgres \
      -c "CREATE DATABASE \"${DB_NAME}\";"
    success "Created database '${DB_NAME}'"
  else
    success "Database '${DB_NAME}' already exists"
  fi
else
  if ! command -v psql &>/dev/null; then
    error "psql not found. Install the PostgreSQL client tools or use Docker Compose."
  fi
  # Build a URL pointing to the maintenance 'postgres' db on the same server
  SERVER_URL="${DATABASE_URL%/${DB_NAME}}"
  EXISTS=$(psql "${SERVER_URL}/postgres" -tAc \
    "SELECT 1 FROM pg_database WHERE datname='${DB_NAME}'" || true)
  if [[ "${EXISTS}" != "1" ]]; then
    psql "${SERVER_URL}/postgres" -c "CREATE DATABASE \"${DB_NAME}\";"
    success "Created database '${DB_NAME}'"
  else
    success "Database '${DB_NAME}' already exists"
  fi
fi

# ── Apply migrations via Diesel ───────────────────────────────────────────────
echo ""
info "Applying migrations (diesel migration run)..."
cd "${PROJECT_ROOT}"
diesel migration run --database-url "${DATABASE_URL}"
echo ""
success "All migrations applied"

# ── Optionally seed demo/test data ───────────────────────────────────────────
if [[ "${SEED}" == "true" ]]; then
  echo ""
  bash "${SCRIPT_DIR}/db_seed.sh"
fi

# ── Summary ───────────────────────────────────────────────────────────────────
echo ""
echo -e "${BLUE}───────────────────────────────────────────────${NC}"
success "Database setup complete"
echo "  Connection : ${DATABASE_URL}"
echo ""
echo "Next steps:"
if [[ "${SEED}" == "false" ]]; then
  echo "  Load demo/test data : ./scripts/db_seed.sh"
fi
echo "  Start Marreq        : cargo run --bin marreq"
echo "  Apply future updates: ./scripts/db_migrate.sh up"
echo "  Backup database     : ./scripts/db_backup.sh"
echo -e "${BLUE}───────────────────────────────────────────────${NC}"
