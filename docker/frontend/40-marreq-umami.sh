#!/bin/sh
# Optional usage statistics with a self-hosted Umami (issue #412).
#
# When MARREQ_UMAMI_WEBSITE_ID is set, add the Umami tracker to the SPA's
# index.html. The script and its endpoint are served by this nginx under
# /stats/ (see default.conf.template), so the browser only talks to Marreq's
# own origin. Unset (the default): index.html is left as built, nothing is sent.
set -eu

index=/usr/share/nginx/html/index.html
marker='data-marreq-umami'

# Start from the built page each time, so a changed or removed id takes effect.
if grep -q "$marker" "$index"; then
  sed -i "/$marker/d" "$index"
fi

id="${MARREQ_UMAMI_WEBSITE_ID:-}"
if [ -z "$id" ]; then
  exit 0
fi
case "$id" in
  *[!0-9a-fA-F-]*)
    echo "40-marreq-umami: MARREQ_UMAMI_WEBSITE_ID must be an Umami website id (a UUID); analytics stay off" >&2
    exit 0
    ;;
esac

tag="    <script defer src=\"/stats/script.js\" data-website-id=\"$id\" data-exclude-search=\"true\" data-do-not-track=\"true\" $marker></script>"
sed -i "s#</head>#$tag\n  </head>#" "$index"
echo "40-marreq-umami: usage statistics enabled (website $id, via /stats/)"
