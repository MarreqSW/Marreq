#!/usr/bin/env bash
# Install the repository-maintained Marreq production backup timer on the VPS.
set -euo pipefail
umask 077

REPO_ROOT="${MARREQ_REPO_ROOT:-/home/marreq/Marreq}"
BACKUP_SCRIPT="${REPO_ROOT}/scripts/marreq-production-backup.sh"
UNIT_DIR="${XDG_CONFIG_HOME:-${HOME}/.config}/systemd/user"
SERVICE="${UNIT_DIR}/marreq-backup.service"
TIMER="${UNIT_DIR}/marreq-backup.timer"
CRON_MARKER="# marreq-production-backup"

[[ -x "${BACKUP_SCRIPT}" || -f "${BACKUP_SCRIPT}" ]] || {
  echo "error: missing ${BACKUP_SCRIPT}" >&2
  exit 1
}
chmod 750 "${BACKUP_SCRIPT}"

mkdir -p "${HOME}/backups/marreq" "${HOME}/backups/logs" "${HOME}/bin"
chmod 700 "${HOME}/backups" "${HOME}/backups/marreq" "${HOME}/backups/logs"

install_systemd_user() {
  mkdir -p "${UNIT_DIR}"
  cat > "${SERVICE}" <<EOF
[Unit]
Description=Marreq routine online production backup
After=docker.service

[Service]
Type=oneshot
Nice=10
IOSchedulingClass=best-effort
IOSchedulingPriority=7
Environment=MARREQ_REPO_ROOT=${REPO_ROOT}
Environment=MARREQ_BACKUP_ROOT=${HOME}/backups/marreq
Environment=MARREQ_OPERATIONS_LOCK=${HOME}/.marreq-operations.lock
ExecStart=${BACKUP_SCRIPT}
EOF

  cat > "${TIMER}" <<EOF
[Unit]
Description=Daily Marreq production backup at 03:00

[Timer]
OnCalendar=*-*-* 03:00:00
Persistent=true
RandomizedDelaySec=5m

[Install]
WantedBy=timers.target
EOF

  systemctl --user daemon-reload
  systemctl --user enable --now marreq-backup.timer
  systemctl --user list-timers marreq-backup.timer --no-pager || true
  echo "installed systemd user timer marreq-backup.timer"
}

install_cron() {
  local tmp
  tmp="$(mktemp)"
  crontab -l 2>/dev/null | grep -v "${CRON_MARKER}" > "${tmp}" || true
  echo "0 3 * * * MARREQ_REPO_ROOT=${REPO_ROOT} ${BACKUP_SCRIPT} ${CRON_MARKER}" >> "${tmp}"
  crontab "${tmp}"
  rm -f "${tmp}"
  echo "installed cron entry for 03:00 local time"
}

if systemctl --user show-environment >/dev/null 2>&1; then
  install_systemd_user
else
  echo "systemd user session unavailable; falling back to cron"
  install_cron
fi

echo "touching shared operations lock"
touch "${HOME}/.marreq-operations.lock"
chmod 600 "${HOME}/.marreq-operations.lock"

if [[ "${1:-}" == "--run-now" ]]; then
  echo "executing first backup"
  "${BACKUP_SCRIPT}"
fi

echo "backup installer complete"
