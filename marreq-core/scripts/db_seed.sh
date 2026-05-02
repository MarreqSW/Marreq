#!/bin/bash
set -euo pipefail

# db_seed.sh — Load demo/test data into the Marreq database
#
# Executes scripts/init_complete.sql, which inserts sample projects, users,
# requirements, tests, and related entities.  The SQL file contains its own
# safety guards and will refuse to run if:
#   • Any required tables are missing (schema not initialised — run db_setup.sh first)
#   • The 'projects' table is non-empty (data already present)
#
# ⚠  FOR TESTING AND DEMO ENVIRONMENTS ONLY — do not run against production data.
#
# Usage:
#   ./marreq-core/scripts/db_seed.sh
#
# Prerequisites:
#   • Database schema already applied (./marreq-core/scripts/db_setup.sh)
#   • DATABASE_URL in .env or environment

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BACKEND_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
REPO_ROOT="$(cd "${BACKEND_ROOT}/.." && pwd)"
COMPOSE_FILE="${REPO_ROOT}/docker/docker-compose.yml"

# ── Colors ───────────────────────────────────────────────────────────────────
GREEN='\033[0;32m'; YELLOW='\033[1;33m'
RED='\033[0;31m';   BLUE='\033[0;34m'; NC='\033[0m'
info()    { echo -e "${BLUE}ℹ  $*${NC}"; }
success() { echo -e "${GREEN}✅ $*${NC}"; }
warn()    { echo -e "${YELLOW}⚠  $*${NC}"; }
error()   { echo -e "${RED}❌ $*${NC}" >&2; exit 1; }

echo -e "${YELLOW}"
echo "==========================================="
echo "   Marreq — Seed Demo/Test Data"
echo "   FOR TESTING AND DEMO ONLY"
echo "==========================================="
echo -e "${NC}"

# ── Load .env ────────────────────────────────────────────────────────────────
if [[ -f "${REPO_ROOT}/.env" ]]; then
  info "Loading ${REPO_ROOT}/.env"
  set -a; source "${REPO_ROOT}/.env"; set +a
fi

DATABASE_URL="${DATABASE_URL:-postgres://rust:rust@127.0.0.1:5433/marreq}"
DB_USER="${DATABASE_URL#*://}"
DB_USER="${DB_USER%%:*}"
DB_NAME="${DATABASE_URL##*/}"
DB_NAME="${DB_NAME%%\?*}"

SEED_SQL="${SCRIPT_DIR}/init_complete.sql"
[[ -f "${SEED_SQL}" ]] || error "Seed file not found: ${SEED_SQL}"

# ── Detect Docker Compose ────────────────────────────────────────────────────
USE_DOCKER=false
DC=""
if docker compose version >/dev/null 2>&1; then
  DC="docker compose -f ${COMPOSE_FILE}"; USE_DOCKER=true
elif docker-compose version >/dev/null 2>&1; then
  DC="docker-compose -f ${COMPOSE_FILE}"; USE_DOCKER=true
fi

# ── Run seed SQL ─────────────────────────────────────────────────────────────
info "Running ${SEED_SQL}..."
echo ""
if [[ "${USE_DOCKER}" == "true" ]]; then
  DB_CID=$(cd "${REPO_ROOT}" && ${DC} ps -q db || true)
  [[ -z "${DB_CID}" ]] && error "The 'db' Docker service is not running. Start it with: ${DC} up -d db"
  docker exec -i "${DB_CID}" psql -U "${DB_USER}" -d "${DB_NAME}" \
    -v ON_ERROR_STOP=1 < "${SEED_SQL}"
else
  command -v psql &>/dev/null || error "psql not found. Install PostgreSQL client tools."
  psql "${DATABASE_URL}" -v ON_ERROR_STOP=1 -f "${SEED_SQL}"
fi

echo ""
success "Demo data loaded into '${DB_NAME}'"
echo ""
echo "Pre-configured accounts (all passwords: ChangeMe123!):"
echo "  alice       — Admin"
echo "  dr_smith    — Admin"
echo "  eng_jones   — User"
echo "  tech_lee    — User"
echo "  qa_wilson   — User"
echo "  admin       — Admin"
echo ""
echo "Projects: Space Project, Marreq Project, Empty Project"
