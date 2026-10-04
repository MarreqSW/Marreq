# Marreq User Manual

This folder contains the end-user documentation for **Marreq** (Requirement Manager).

## Contents

- **[user-manual.md](user-manual.md)** — Full user manual: sign-in, dashboard and navigation, projects and groups, requirements, verifications, traceability (matrix, hierarchy, DSM), baselines, catalog, reports and exports, import, members and reviewers, account, and administration.
- **[workflow.md](workflow.md)** — Typical workflow with Marreq: project setup → requirements → verifications → traceability → approval → baselines → export.
- **[doors-to-marreq-migration.md](doors-to-marreq-migration.md)** — For IBM DOORS users: concept mapping, workflow comparison, and migration path to Marreq (ReqIF/Excel).

Use the table of contents inside `user-manual.md` to jump to a specific section.

## HTML versions and screenshots

- **[index.html](index.html)** links the HTML versions in `generated/`. After changing a Markdown file or a screenshot, rebuild them with pandoc:

  ```bash
  docs/user-manual/build.sh
  ```

  The script reads GitHub-flavoured Markdown (so the manual's anchors work), links `styles.css`, and rewrites links and image paths for `generated/` (`build-links.lua`). Commit the regenerated files with the Markdown change.
- **[screenshots/](screenshots/README.md)** are captured with `capture_screenshots.mjs` against a throwaway instance with the demo data; see that folder's README.
