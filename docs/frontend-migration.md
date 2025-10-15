# Frontend Migration Report

_Date: 2025-10-15_

## Asset Restructure Summary

- **CSS**
  - Moved legacy flat styles into ITCSS-inspired folders (`00-settings`, `20-generic`, `50-components`, `60-utilities`, `pages/`).
  - Added `css/index.css` as the single entry point imported by `reqman.css`.
  - Introduced `_graveyard.css` for parking rules pending deletion.

- **JavaScript**
  - Deleted monolithic `reqman.js`; replaced with ESM bootstrap (`js/app.js`), shared core utilities, feature modules, and page controllers.
  - Added `js/theme-prefetch.js` to preserve pre-render theme synchronisation.

- **Templates**
  - Inserted doc comments describing CSS/JS dependencies to aid maintainability and purge safelists.
  - Replaced inline scripts/handlers with data attributes and delegated listeners.
  - Added `data-page` hooks for the automatic controller loader.

## Newly Added Files

| Path | Purpose |
| --- | --- |
| `src/html/static/css/index.css` | Layered CSS entry point. |
| `src/html/static/css/00-settings/*` ... `pages/*` | Organised CSS layers (tokens, components, page styles). |
| `src/html/static/css/_graveyard.css` | Holding area for deprecated selectors. |
| `src/html/static/js/core/*` | DOM + network helpers. |
| `src/html/static/js/modules/*` | Thematic feature modules (theme, sidebar, modals, inline edit, etc.). |
| `src/html/static/js/pages/*` | Page-specific controllers. |
| `src/html/static/js/app.js` | Entry point that wires global behaviours and lazy-loads controllers. |
| `src/html/static/js/theme-prefetch.js` | Early theme application script. |
| `.eslintrc.json` / `.stylelintrc.json` | Linting configuration files. |
| `docs/frontend-coverage.md` | Coverage & safelist notes. |
| `docs/frontend-smoke-test.md` | Manual smoke test checklist. |

## Removed Files

- `src/html/static/reqman.js`
- `src/html/static/css/{base.css, buttons.css, cards.css, components.css, forms.css, navigation.css, tables.css, tokens.css, tree.css, utilities.css}` (superseded by layered structure)

## Unused / Legacy Selectors Identified

| Location | Candidate | Action |
| --- | --- | --- |
| `css/50-components/dashboard.css` | `.reqman-placeholder-feature*`, `.reqman-project-card--bg-*` | Verify with product; move to `_graveyard.css` when confirmed unused. |
| `css/60-utilities/utilities.css` | `.mt-xxl`, `.mb-xxl`, `.pt-xxl`, `.pb-xxl` | Replace with spacing tokens or remove once referenced audit complete. |
| `css/pages/reports.css` | `.reqman-report-*` variants | Reports view currently disabled; mark for removal post stakeholder review. |

## JavaScript Deletions / Obsolete API Calls

- Inline global functions (`deleteRequirement`, `deleteTest`, `generateBackup`, etc.) superseded by modular controllers and `deleteActions` helper.
- Theme initialiser embedded in `layout.html.hbs` replaced with `theme-prefetch.js` + `modules/theme.js`.

## Safelist Used for PurgeCSS

```
Regex: ^status-, ^is-, ^js-
Bootstrap: btn, card, row, col-*, d-*, text-*, bg-*, alert-*, modal-*, collapse, show, fade, dropdown, nav, navbar, table, form-*
State: [aria-*], .active, .show, .collapsed, .disabled
Routes: body[data-page] (requirements, tests, matrix, categories, requirement-form, test-form, admin-cache-stats, admin-cache-health, admin-backup, log-analytics, logs, entity-logs, map-columns, applicability, login)
```

## Outstanding Follow-ups

1. Install lint tooling: `npm install --save-dev eslint eslint-plugin-import stylelint stylelint-config-standard`.
2. Run PurgeCSS with the safelist above and move confirmed-unused selectors into `_graveyard.css`.
3. Schedule QA run with `docs/frontend-smoke-test.md` before deploying the refactor.
4. Consider splitting remaining large CSS (e.g. `dashboard.css`) into smaller component files once behaviour stabilises.
