#!/usr/bin/env bash
# Routine online daily backup for the Marreq production VPS.
#
# This is NOT a coherent pre-migration recovery bundle. Application writes may
# continue while dumps and attachment copies run. For schema changes, stop
# writers and use marreq-backup instead (see docs/developer/database-migrations.md).
set -euo pipefail
umask 077

REPO_ROOT="${MARREQ_REPO_ROOT:-/home/marreq/Marreq}"
BACKUP_ROOT="${MARREQ_BACKUP_ROOT:-/home/marreq/backups/marreq}"
LOCK_PATH="${MARREQ_OPERATIONS_LOCK:-/home/marreq/.marreq-operations.lock}"
COMPOSE_PROJECT="${MARREQ_COMPOSE_PROJECT:-docker}"
COMPOSE_FILES=(
  "${REPO_ROOT}/docker/docker-compose.light.github.yml"
  "${REPO_ROOT}/docker/docker-compose.persistence.yml"
  "${REPO_ROOT}/docker/docker-compose.release.yml"
)
MIN_DISK_MB="${MARREQ_MIN_DISK_MB:-4096}"
KEEP_DAILY="${MARREQ_BACKUP_KEEP_DAILY:-7}"
KEEP_WEEKLY="${MARREQ_BACKUP_KEEP_WEEKLY:-4}"
LOG_DIR="${MARREQ_BACKUP_LOG_DIR:-/home/marreq/backups/logs}"
OFFSITE_CMD="${MARREQ_BACKUP_OFFSITE_CMD:-}"

mkdir -p "${BACKUP_ROOT}" "${LOG_DIR}"
chmod 700 "${BACKUP_ROOT}" "${LOG_DIR}"

TIMESTAMP="$(date +%Y%m%dT%H%M%S%z)"
STAGING="$(mktemp -d "${BACKUP_ROOT}/.incomplete-${TIMESTAMP}-XXXXXX")"
FINAL="${BACKUP_ROOT}/daily-${TIMESTAMP}"
LOG_FILE="${LOG_DIR}/backup-${TIMESTAMP}.log"
LOCK_FD=9

cleanup() {
  local code=$?
  if [[ -d "${STAGING}" ]]; then
    rm -rf "${STAGING}"
  fi
  if [[ ${code} -ne 0 ]]; then
    echo "backup FAILED at $(date --iso-8601=seconds)" | tee -a "${LOG_FILE}" >&2
  fi
  exit "${code}"
}
trap cleanup EXIT

exec 9>"${LOCK_PATH}"
if ! flock -n 9; then
  echo "error: deploy/backup lock busy (${LOCK_PATH})" >&2
  exit 1
fi
echo "pid=$$ kind=backup-online started=$(date --iso-8601=seconds)" >&9

exec > >(tee -a "${LOG_FILE}") 2>&1
echo "Marreq online backup starting ${TIMESTAMP}"
echo "NOTE: online dump + attachment copy is not a coherent pre-migration recovery bundle"

free_mb="$(df -Pm "${BACKUP_ROOT}" | awk 'NR==2 {print $4}')"
if [[ "${free_mb}" -lt "${MIN_DISK_MB}" ]]; then
  echo "error: insufficient disk space (${free_mb} MiB < ${MIN_DISK_MB} MiB)" >&2
  exit 1
fi

compose=(docker compose -p "${COMPOSE_PROJECT}")
for file in "${COMPOSE_FILES[@]}"; do
  [[ -f "${file}" ]] || { echo "error: missing compose file ${file}" >&2; exit 1; }
  compose+=(-f "${file}")
done

DB_CID="$("${compose[@]}" ps -q db)"
[[ -n "${DB_CID}" ]] || { echo "error: db container not running" >&2; exit 1; }

mkdir -p "${STAGING}/postgres" "${STAGING}/attachments"
chmod 700 "${STAGING}" "${STAGING}/postgres" "${STAGING}/attachments"

echo "dumping database marreq"
docker exec "${DB_CID}" pg_dump -U rust -d marreq --no-password --format=custom --compress=9 \
  > "${STAGING}/postgres/marreq.dump"
echo "dumping database umami"
docker exec "${DB_CID}" pg_dump -U rust -d umami --no-password --format=custom --compress=9 \
  > "${STAGING}/postgres/umami.dump" || {
    echo "warning: umami dump failed; checking whether database exists"
    if docker exec "${DB_CID}" psql -U rust -d postgres -Atc "SELECT 1 FROM pg_database WHERE datname='umami'" | grep -q 1; then
      echo "error: umami database exists but dump failed" >&2
      exit 1
    fi
    echo "umami database absent; continuing"
    rm -f "${STAGING}/postgres/umami.dump"
  }
echo "dumping PostgreSQL globals"
docker exec "${DB_CID}" pg_dumpall -U rust --globals-only --no-password \
  > "${STAGING}/postgres/globals.sql"

SERVER_CID="$("${compose[@]}" ps -q marreq-server)"
[[ -n "${SERVER_CID}" ]] || { echo "error: marreq-server container not running" >&2; exit 1; }

echo "copying attachment files"
# Empty stores are valid, but a failed docker cp must not be reported as success.
attach_err="$(mktemp)"
set +e
docker exec "${SERVER_CID}" sh -c 'test -d /var/lib/marreq/attachments' >/dev/null 2>"${attach_err}"
attach_dir_status=$?
set -e
if [[ ${attach_dir_status} -ne 0 ]]; then
  echo "error: attachment directory missing inside marreq-server" >&2
  cat "${attach_err}" >&2 || true
  rm -f "${attach_err}"
  exit 1
fi
rm -f "${attach_err}"
# Count entries before copy so an empty store stays distinguishable from copy failure.
attach_count="$(docker exec "${SERVER_CID}" sh -c 'find /var/lib/marreq/attachments -mindepth 1 | wc -l' | tr -d '[:space:]')"
if [[ "${attach_count}" == "0" ]]; then
  echo "attachment store empty; recording empty attachments directory"
  mkdir -p "${STAGING}/attachments"
else
  if ! docker cp "${SERVER_CID}:/var/lib/marreq/attachments/." "${STAGING}/attachments/"; then
    echo "error: attachment copy failed" >&2
    exit 1
  fi
  copied_count="$(find "${STAGING}/attachments" -mindepth 1 | wc -l | tr -d '[:space:]')"
  if [[ "${copied_count}" -lt 1 ]]; then
    echo "error: attachment copy produced no files despite non-empty source" >&2
    exit 1
  fi
fi

cat > "${STAGING}/backup-meta.json" <<EOF
{
  "format": "marreq.online-backup.v1",
  "kind": "routine-online",
  "not_coherent_recovery_bundle": true,
  "created_at": "$(date --iso-8601=seconds)",
  "host": "$(hostname -f 2>/dev/null || hostname)",
  "compose_project": "${COMPOSE_PROJECT}",
  "databases": ["marreq", "umami", "globals"]
}
EOF

echo "checksumming"
(
  cd "${STAGING}"
  find . -type f ! -name 'SHA256SUMS' -print0 | sort -z | xargs -0 sha256sum > SHA256SUMS
  sha256sum -c SHA256SUMS >/dev/null
)

chmod -R go-rwx "${STAGING}"
mv "${STAGING}" "${FINAL}"
STAGING=""
echo "published ${FINAL}"

# Retention: keep newest daily snapshots and one weekly per ISO week.
mapfile -t dailies < <(ls -1dt "${BACKUP_ROOT}"/daily-* 2>/dev/null || true)
keep_set=()
weekly_seen=()
index=0
for path in "${dailies[@]}"; do
  base="$(basename "${path}")"
  # daily-YYYYMMDDTHHMMSS+ZZZZ
  day="${base#daily-}"
  day="${day:0:8}"
  if [[ ${index} -lt ${KEEP_DAILY} ]]; then
    keep_set+=("${path}")
  else
    week="$(date -d "${day:0:4}-${day:4:2}-${day:6:2}" +%G-W%V 2>/dev/null || echo "")"
    if [[ -n "${week}" ]] && [[ ! " ${weekly_seen[*]} " =~ " ${week} " ]] && [[ ${#weekly_seen[@]} -lt ${KEEP_WEEKLY} ]]; then
      weekly_seen+=("${week}")
      keep_set+=("${path}")
    else
      # Only delete completed daily-* directories owned by this layout.
      if [[ -f "${path}/SHA256SUMS" ]] && [[ -f "${path}/backup-meta.json" ]]; then
        echo "retaining-policy: removing ${path}"
        rm -rf "${path}"
      fi
    fi
  fi
  index=$((index + 1))
done

if [[ -n "${OFFSITE_CMD}" ]]; then
  echo "running configured off-site backup command"
  # Intentionally opaque: operator supplies a local wrapper; credentials stay off-repo.
  bash -c "${OFFSITE_CMD}" -- "${FINAL}"
else
  echo "off-site backup not configured (set MARREQ_BACKUP_OFFSITE_CMD); local retention only"
fi

echo "backup OK ${FINAL}"
