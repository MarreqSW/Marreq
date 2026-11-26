#!/usr/bin/env bash
# Selective production deployment for a coordinated Marreq product release.
# See docs/developer/continuous-deployment.md
set -euo pipefail
umask 077

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec python3 "${ROOT}/deploy_release.py" "$@"
