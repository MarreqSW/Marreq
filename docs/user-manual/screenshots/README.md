# User Manual Screenshots

Screenshots of the SPA for the [User Manual](../user-manual.md), captured with
`capture_screenshots.mjs` (Playwright, 1280×800, light theme) as `alice` in the
*Space Project* demo data.

To recapture:

1. Start a **throwaway** database and backend with the demo data:
   `marreq-core/scripts/init_complete.sql`, plus
   `seed_space_project_bulk_requirements.sql` and
   `seed_space_project_bulk_tests_and_hierarchy.sql` for a fuller project.
   Add a project reviewer and a couple of baselines.
2. Start the SPA against that backend (in `frontend/`):
   `MARREQ_API_PROXY_TARGET=http://127.0.0.1:<backend port> npx vite --port <port>`
3. From the repository root:
   `MARREQ_URL=http://127.0.0.1:<port> npm run screenshots:manual`
   (add `MARREQ_CHROME=/usr/bin/google-chrome` if Playwright's browser is not installed).

| File | Page |
|------|------|
| login.png | Sign-in page |
| dashboard.png | Project dashboard (Project overview) |
| requirements-list.png | Requirements, Table view |
| requirement-detail.png | Requirement detail page |
| requirement-create.png | Create requirement form (bottom, with the Create button) |
| verifications-list.png | Verifications list |
| matrix.png | Traceability › Matrix |
| hierarchy.png | Traceability › Hierarchy with a selected node and **Add child** |
| dsm.png | Traceability › DSM |
| baselines-list.png | Baselines (Create baseline form and list) |
| reports.png | Reports & exports (Coverage & gaps) |
| settings-members.png | Project settings › Members & reviewers |

After changing screenshots or the manual, rebuild the HTML with `../build.sh`.
