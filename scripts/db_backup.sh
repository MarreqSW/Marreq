#!/bin/bash
set -euo pipefail

# db_backup.sh — PostgreSQL backup utility for Marreq
#
# Runs pg_dump inside the database container and writes a compressed SQL
# archive to ./backups/.  The output filename includes a timestamp so runs
# never overwrite each other.
#
# Usage:
#   ./scripts/db_backup.sh                      # saves to ./backups/
#   ./scripts/db_backup.sh /path/to/output.sql.gz  # custom output path
#
# Prerequisites:
#   • Docker running with the 'db' service up, OR pg_dump available locally.
#   • DATABASE_URL in .env or environment.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "${SCRIPT_DIR}")"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
DEFAULT_BACKUP_DIR="${PROJECT_ROOT}/backups"
OUTPUT="${1:-${DEFAULT_BACKUP_DIR}/marreq_${TIMESTAMP}.sql.gz}"

# ── Colors ───────────────────────────────────────────────────────────────────
GREEN='\033[0;32m'; YELLOW='\033[1;33m'
RED='\033[0;31m';   BLUE='\033[0;34m'; NC='\033[0m'
info()    { echo -e "${BLUE}ℹ  $*${NC}"; }
success() { echo -e "${GREEN}✅ $*${NC}"; }
error()   { echo -e "${RED}❌ $*${NC}" >&2; exit 1; }

echo -e "${BLUE}"
echo "==========================================="
echo "   Marreq — Database Backup"
echo "==========================================="
echo -e "${NC}"

# ── Load .env ────────────────────────────────────────────────────────────────
if [[ -f "${PROJECT_ROOT}/.env" ]]; then
  info "Loading ${PROJECT_ROOT}/.env"
  set -a; source "${PROJECT_ROOT}/.env"; set +a
fi

DATABASE_URL="${DATABASE_URL:-postgres://rust:rust@127.0.0.1:5432/marreq}"
DB_USER="${DATABASE_URL#*://}"
DB_USER="${DB_USER%%:*}"
DB_NAME="${DATABASE_URL##*/}"
DB_NAME="${DB_NAME%%\?*}"

# ── Ensure output directory exists ───────────────────────────────────────────
mkdir -p "$(dirname "${OUTPUT}")"

# ── Detect Docker Compose ────────────────────────────────────────────────────
USE_DOCKER=false
DC=""
if docker compose version >/dev/null 2>&1; then
  DC="docker compose"; USE_DOCKER=true
elif docker-compose version >/dev/null 2>&1; then
  DC="docker-compose"; USE_DOCKER=true
fi

# ── Run pg_dump ───────────────────────────────────────────────────────────────
info "Backing up '${DB_NAME}' → ${OUTPUT}"
echo ""

if [[ "${USE_DOCKER}" == "true" ]]; then
  DB_CID=$(cd "${PROJECT_ROOT}" && ${DC} ps -q db || true)
  [[ -z "${DB_CID}" ]] && error "The 'db' Docker service is not running. Start it with: ${DC} up -d db"
  docker exec "${DB_CID}" pg_dump -U "${DB_USER}" -d "${DB_NAME}" --no-password | gzip > "${OUTPUT}"
else
  command -v pg_dump &>/dev/null || error "pg_dump not found. Install PostgreSQL client tools."
  pg_dump "${DATABASE_URL}" | gzip > "${OUTPUT}"
fi

BYTES=$(du -sh "${OUTPUT}" | cut -f1)
echo ""
success "Backup complete: ${OUTPUT} (${BYTES})"
echo ""
echo "Restore with:"
if [[ "${USE_DOCKER}" == "true" ]]; then
  echo "  gunzip -c ${OUTPUT} | docker exec -i <container> psql -U ${DB_USER} -d ${DB_NAME}"
else
  echo "  gunzip -c ${OUTPUT} | psql \"${DATABASE_URL}\""
fi
