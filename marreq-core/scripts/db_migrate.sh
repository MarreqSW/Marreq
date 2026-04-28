#!/bin/bash
set -euo pipefail

# db_migrate.sh — Diesel migration management wrapper
#
# Thin wrapper around diesel migration commands.  Use this to apply pending
# migrations after pulling a new version of Marreq, or to revert migrations
# during development.
#
# Usage:
#   ./scripts/db_migrate.sh up           # apply all pending migrations
#   ./scripts/db_migrate.sh down [N]     # revert N migrations (default: 1)
#   ./scripts/db_migrate.sh list         # show applied / pending migrations
#   ./scripts/db_migrate.sh redo         # revert 1, then re-apply 1 (dev helper)
#
# Prerequisites:
#   • diesel CLI: cargo install diesel_cli --no-default-features --features postgres
#   • DATABASE_URL in .env or environment

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BACKEND_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
REPO_ROOT="$(cd "${BACKEND_ROOT}/.." && pwd)"

# ── Colors ───────────────────────────────────────────────────────────────────
GREEN='\033[0;32m'; YELLOW='\033[1;33m'
RED='\033[0;31m';   BLUE='\033[0;34m'; NC='\033[0m'
info()    { echo -e "${BLUE}ℹ  $*${NC}"; }
success() { echo -e "${GREEN}✅ $*${NC}"; }
warn()    { echo -e "${YELLOW}⚠  $*${NC}"; }
error()   { echo -e "${RED}❌ $*${NC}" >&2; exit 1; }

SUBCOMMAND="${1:-up}"
N="${2:-1}"

# ── Load .env ────────────────────────────────────────────────────────────────
if [[ -f "${REPO_ROOT}/.env" ]]; then
  set -a; source "${REPO_ROOT}/.env"; set +a
fi

DATABASE_URL="${DATABASE_URL:-postgres://rust:rust@127.0.0.1:5433/marreq}"

# ── Require diesel CLI ───────────────────────────────────────────────────────
if ! command -v diesel &>/dev/null; then
  error "diesel CLI not found.
  Install it with:
    cargo install diesel_cli --no-default-features --features postgres"
fi

cd "${BACKEND_ROOT}"

case "${SUBCOMMAND}" in
  up)
    info "Applying all pending migrations..."
    diesel migration run --database-url "${DATABASE_URL}"
    success "Migrations up to date"
    ;;
  down)
    warn "Reverting ${N} migration(s) — this may be destructive."
    diesel migration revert --number "${N}" --database-url "${DATABASE_URL}"
    success "Reverted ${N} migration(s)"
    ;;
  list)
    info "Migration status (${DATABASE_URL}):"
    echo ""
    diesel migration list --database-url "${DATABASE_URL}"
    echo ""
    echo "  [X] = applied   [ ] = pending"
    ;;
  redo)
    warn "Redo: reverting 1 migration and re-applying it..."
    diesel migration redo --database-url "${DATABASE_URL}"
    success "Redo complete"
    ;;
  *)
    echo "Usage: $0 {up|down [N]|list|redo}"
    exit 1
    ;;
esac
