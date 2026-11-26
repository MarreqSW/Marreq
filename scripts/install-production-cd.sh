#!/usr/bin/env bash
# Operator installer for production CD scripts on the Marreq VPS.
# Safe to re-run. Does not overwrite Compose customizations or .env files.
set -euo pipefail
umask 077

REPO_ROOT="${MARREQ_REPO_ROOT:-/home/marreq/Marreq}"
BIN_DIR="${HOME}/bin"
ENTRY="${BIN_DIR}/marreq-deploy-entry"
ACTIONS_PUBKEY="${1:-}"

[[ -d "${REPO_ROOT}/scripts" ]] || {
  echo "error: ${REPO_ROOT}/scripts missing" >&2
  exit 1
}

mkdir -p "${BIN_DIR}" "${HOME}/backups/marreq" "${HOME}/backups/logs" "${REPO_ROOT}/releases"
chmod 700 "${HOME}/backups" "${HOME}/backups/marreq" "${HOME}/backups/logs" "${REPO_ROOT}/releases"
touch "${HOME}/.marreq-operations.lock"
chmod 600 "${HOME}/.marreq-operations.lock"

chmod 750 \
  "${REPO_ROOT}/scripts/deploy-release.sh" \
  "${REPO_ROOT}/scripts/deploy_release.py" \
  "${REPO_ROOT}/scripts/production-deploy-entry.sh" \
  "${REPO_ROOT}/scripts/marreq-production-backup.sh" \
  "${REPO_ROOT}/scripts/install-production-backup.sh" \
  "${REPO_ROOT}/scripts/install-production-cd.sh"

install -m 750 "${REPO_ROOT}/scripts/production-deploy-entry.sh" "${ENTRY}"
# Point the entrypoint at this checkout.
sed -i "s|^DEPLOY_SCRIPT=.*|DEPLOY_SCRIPT=\"${REPO_ROOT}/scripts/deploy-release.sh\"|" "${ENTRY}" || true

if [[ -n "${ACTIONS_PUBKEY}" ]]; then
  AUTH="${HOME}/.ssh/authorized_keys"
  mkdir -p "${HOME}/.ssh"
  chmod 700 "${HOME}/.ssh"
  touch "${AUTH}"
  chmod 600 "${AUTH}"
  options='command="/home/marreq/bin/marreq-deploy-entry",no-agent-forwarding,no-port-forwarding,no-X11-forwarding,no-pty'
  # Replace any previous Actions deploy key line with the same comment.
  grep -v 'github-actions-marreq-production-deploy' "${AUTH}" > "${AUTH}.tmp" || true
  echo "${options} ${ACTIONS_PUBKEY}" >> "${AUTH}.tmp"
  mv "${AUTH}.tmp" "${AUTH}"
  chmod 600 "${AUTH}"
  echo "authorized Actions deploy key with forced-command"
else
  echo "skipped authorized_keys update (pass pubkey as \$1)"
fi

"${REPO_ROOT}/scripts/install-production-backup.sh"

echo "production CD installer complete"
echo "next: scripts/deploy-release.sh --dry-run v0.2.1"
