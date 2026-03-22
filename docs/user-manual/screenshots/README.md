# Screenshots for the User Manual

The User Manual references these screenshots. To generate them:

1. **Start Marreq** (database and app):
   - `docker compose -f docker/docker-compose.yml up -d db`
   - `./scripts/db_setup.sh --seed` (if needed)
   - `cargo run --bin marreq`

2. **Install Playwright** (one-time):
   - `npm install`
   - `npx playwright install chromium`

3. **Capture screenshots** (from the repo root):
   - `node docs/user-manual/capture_screenshots.mjs`

Screenshots are saved here as `login.png`, `home.png`, `projects.png`, `project-detail.png`, `requirements-list.png`, `requirement-detail.png`, `tests-list.png` (verifications list), `matrix.png`, `baselines-list.png`, and `reports.png`. The script uses a namespace-style project base path and defaults to `/dr_smith/space-project`. Optional env vars: `MARREQ_URL` (default `http://localhost:8000`), `MARREQ_USER`, `MARREQ_PASS`, and `MARREQ_PROJECT_BASE_PATH`.
