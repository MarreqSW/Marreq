cd "$(dirname "$0")"

MARREQ_MODE=draft_write \
MARREQ_BASE_URL=http://localhost:8000  \
MARREQ_API_TOKEN=e55b080f577c1d53e9393ea36caae49755362e6aa294a401e1ead7eb5bc403c7  \
MARREQ_PROJECT_ID=4  \
npm start 
