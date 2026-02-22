# Screenshots for the User Manual

The User Manual references these screenshots. To generate them:

1. **Start ReqMan** (database and app):
   - `docker compose up -d db`
   - `./scripts/setup_database.sh` (if needed)
   - `cargo run --bin req_man`

2. **Install Playwright** (one-time):
   - `npm install`
   - `npx playwright install chromium`

3. **Capture screenshots** (from the repo root):
   - `node doc/UserManual/capture_screenshots.mjs`

Screenshots are saved here as `login.png`, `home.png`, `projects.png`, `project-detail.png`, `requirements-list.png`, `requirement-detail.png`, `tests-list.png`, `matrix.png`, `baselines-list.png`, and `reports.png`. Optional env vars: `REQMAN_URL` (default `http://localhost:8000`), `REQMAN_USER`, `REQMAN_PASS`.
