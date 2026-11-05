#!/usr/bin/env bash
set -euo pipefail
exec cargo run --quiet --locked -p marreq-contracts -- verify "$@"
