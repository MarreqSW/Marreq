#!/bin/sh
set -eu
# Schema validation happens in both backend binaries before binding a socket.
# Initialization and demo seeding are explicit development operations.
if [ "${MARREQ_AUTO_SEED:-false}" != "false" ]; then
  echo 'Automatic seeding is disabled; use marreq-core/scripts/db_seed.sh explicitly in development.' >&2
  exit 1
fi
exec "$@"
