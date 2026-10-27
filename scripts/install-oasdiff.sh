#!/usr/bin/env bash
# Pinned upstream standards engine, used only by developer/CI Rust tooling.
set -euo pipefail
[[ $(uname -s) == Linux && $(uname -m) == x86_64 ]] || { echo 'Install oasdiff 1.34.0 for your platform and set OASDIFF.' >&2; exit 1; }
destination=${1:-"$HOME/.cache/marreq-contracts"}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
curl --fail --silent --show-error --location --proto '=https' --tlsv1.2 \
  https://github.com/oasdiff/oasdiff/releases/download/v1.34.0/oasdiff_1.34.0_linux_amd64.tar.gz \
  --output "$work/oasdiff.tar.gz"
printf '%s  %s\n' 7c88e09fd500b231af2819eb21f6fdfc7bcdfe2741d2c1937c59e7b72c8a575a "$work/oasdiff.tar.gz" | sha256sum --check
mkdir -p "$destination"
tar -xzf "$work/oasdiff.tar.gz" -C "$work" oasdiff
install -m 755 "$work/oasdiff" "$destination/oasdiff"
if [[ -n ${GITHUB_PATH:-} ]]; then printf '%s\n' "$destination" >> "$GITHUB_PATH"; fi
