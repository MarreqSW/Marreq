#!/usr/bin/env bash
# Build the HTML versions of the user manual into generated/ (needs pandoc).
#
#   docs/user-manual/build.sh
#
# The Markdown is read as GitHub-flavoured Markdown, so the headings get the
# same anchors as on GitHub and the manual's own table of contents works.
set -euo pipefail
cd "$(dirname "$0")"
mkdir -p generated

build() { # <name> <page title>
  pandoc "$1.md" -f gfm -t html5 --standalone \
    --metadata pagetitle="$2" \
    --css ../styles.css \
    --lua-filter build-links.lua \
    -o "generated/$1.html"
  echo "generated/$1.html"
}

build user-manual "Marreq User Manual"
build workflow "Typical Workflow with Marreq"
build doors-to-marreq-migration "Migrating from DOORS to Marreq"
