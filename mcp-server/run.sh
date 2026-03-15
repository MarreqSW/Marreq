cd "$(dirname "$0")"

MARREQ_MODE=draft_write \
MARREQ_BASE_URL=http://localhost:8000  \
MARREQ_API_TOKEN="${MARREQ_API_TOKEN:?Set MARREQ_API_TOKEN to your personal API token (see docs/developer/mcp-setup.md)}"  \
MARREQ_PROJECT_ID="${MARREQ_PROJECT_ID:?Set MARREQ_PROJECT_ID to your project ID}"  \
npm start 
