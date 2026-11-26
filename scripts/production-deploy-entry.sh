#!/usr/bin/env bash
# Restricted SSH forced-command entrypoint for GitHub Actions production deploys.
#
# Install on the VPS as /home/marreq/bin/marreq-deploy-entry and authorize the
# Actions deploy public key with:
#   command="/home/marreq/bin/marreq-deploy-entry",no-agent-forwarding,no-port-forwarding,no-X11-forwarding,no-pty ssh-ed25519 AAAA...
set -euo pipefail
umask 077

DEPLOY_SCRIPT="${MARREQ_DEPLOY_SCRIPT:-/home/marreq/Marreq/scripts/deploy-release.sh}"
ORIGINAL="${SSH_ORIGINAL_COMMAND:-}"

usage() {
  echo "allowed commands:" >&2
  echo "  deploy vX.Y.Z" >&2
  echo "  deploy --dry-run vX.Y.Z" >&2
  echo "  rollback vX.Y.Z" >&2
  echo "  rollback --dry-run vX.Y.Z" >&2
  exit 2
}

[[ -n "${ORIGINAL}" ]] || usage

# shellcheck disable=SC2206
tokens=( ${ORIGINAL} )
[[ ${#tokens[@]} -ge 2 && ${#tokens[@]} -le 3 ]] || usage

action="${tokens[0]}"
case "${action}" in
  deploy)
    if [[ ${#tokens[@]} -eq 3 ]]; then
      [[ "${tokens[1]}" == "--dry-run" ]] || usage
      tag="${tokens[2]}"
      exec bash "${DEPLOY_SCRIPT}" --dry-run "${tag}"
    fi
    tag="${tokens[1]}"
    exec bash "${DEPLOY_SCRIPT}" "${tag}"
    ;;
  rollback)
    if [[ ${#tokens[@]} -eq 3 ]]; then
      [[ "${tokens[1]}" == "--dry-run" ]] || usage
      tag="${tokens[2]}"
      exec bash "${DEPLOY_SCRIPT}" --rollback --dry-run "${tag}"
    fi
    tag="${tokens[1]}"
    exec bash "${DEPLOY_SCRIPT}" --rollback "${tag}"
    ;;
  *)
    usage
    ;;
esac
