# Frontend Coverage

_Last updated: 2025-10-15_

## CSS Selector Overview

| File | Approx. selectors | Potentially unused | Notes |
| --- | ---:| ---:| --- |
| `css/index.css` | 0 | 0 | Entry file that stitches the layer stack; no selectors of its own. |
| `css/00-settings/tokens.css` | ~120 variables | 0 | Tokens consumed throughout the stack; keep all for design system. |
| `css/50-components/dashboard.css` | ~160 | ~18 | Candidate clean-up: legacy `.reqman-placeholder-feature*`, `.reqman-project-card--bg-*` gradient variants unused in templates after audit. |
| `css/50-components/cards.css` | ~70 | 0 | Shared card styles used by dashboard, requirements, tests, categories. |
| `css/50-components/components.css` | ~55 | ~6 | Old footer + theme toggle primitives kept for backwards compatibility (moved to `_graveyard.css` when confirmed). |
| `css/50-components/tree.css` | ~45 | 0 | Consumed by requirements tree view; dynamic state handled via JS safelist. |
| `css/60-utilities/utilities.css` | ~40 | ~5 | Legacy spacing helpers (`.mt-xxl`, `.mb-xxl`) not referenced; parked for confirmation. |
| `css/pages/dashboard.css` | ~65 | ~8 | Review hero/analytics blocks after product sign-off; several `.reqman-analytics-*` selectors not rendered. |
| `css/pages/projects.css` | ~40 | ~10 | `.project-card` still loaded globally; audit once project cards fully migrated to new modules. |
| `css/pages/categories.css` | ~35 | 0 | Driven by new categories page controller; all selectors exercised. |
| `css/pages/reports.css` | ~25 | ~4 | Reports UI currently hidden; leave until decision to deprecate page. |
| `css/_graveyard.css` | 0 | 0 | Holding file for future removals; intentionally empty. |

> **Next step**: run `stylelint --custom-syntax postcss-html "templates/**/*.hbs"` followed by PurgeCSS (with safelist below) to validate the estimates.

### Safelist (CSS Purge)

- Regex: `^status-`, `^is-`, `^js-`
- Bootstrap classes: `btn`, `card`, `row`, `col-*`, `d-*`, `text-*`, `bg-*`, `alert-*`, `modal`, `collapse`, `show`, `fade`, `dropdown`, `nav`, `navbar`, `table`, `form-*`
- Attribute/state: `[aria-*]`, `.active`, `.show`, `.collapsed`, `.disabled`
- Route markers: `body[data-page]` values (`requirements`, `tests`, `matrix`, `categories`, `requirement-form`, `test-form`, `admin-cache-*`, `log-analytics`, `logs`, `entity-logs`, `map-columns`, `applicability`, `login`)

## JavaScript Module Inventory

| Module | Purpose | Notes |
| --- | --- | --- |
| `js/app.js` | Bootstrapper & global registries | Handles delete actions, confirmation prompts, history back navigation, page controller loading. |
| `js/core/dom.js` | DOM helpers (`$`, `$$`, `on`) | Shared utility for modules/controllers. |
| `js/core/net.js` | Fetch wrappers | Standardises JSON fetch, POST/PATCH helpers. |
| `js/modules/theme.js` | Theme toggle management | Works with `theme-prefetch.js` to keep FOUC-free theme switching. |
| `js/modules/sidebar.js` | Sidebar persistence/responsiveness | Replaces inline sidebar logic. |
| `js/modules/notifications.js` | Toast alerts | Common success/error messaging. |
| `js/modules/inlineEdit.js` | Table inline editing | Used by requirements/tests controllers. |
| `js/modules/sortTable.js` | Client-side table sorting | Provides toggleable sort arrows. |
| `js/modules/modals.js` | Modal form binding | Handles add requirement/test flows. |
| `js/modules/projectSelector.js` | Project picker cookie + navigation | Maintains cross-page project state. |
| `js/modules/deleteActions.js` | Confirmed DELETE requests | Consumed globally through `app.js`. |
| `js/modules/tableFilter.js` | Search filters | Powers categories quick filter. |
| `js/modules/tree.js` | Requirements tree controls | Adds expand/collapse behavior. |
| `js/modules/scrollIndicator.js` | Horizontal scroll thumb | Used on matrix table. |
| `js/modules/diffModal.js` | Log diff rendering | Shared by `logs` and `entity_logs`. |
| `js/modules/referenceValidator.js` | Requirement reference checks | Prevents invalid `REQ-TAG-N` references. |
| `js/modules/multiSelect.js` | Multi-select utility | Currently used by test edit pages. |

No unused modules detected after migration; legacy `reqman.js` removed.

## Follow-up Actions

1. Run PurgeCSS with the safelist above; migrate selectors flagged as unused into `_graveyard.css`.
2. Wire Stylelint/ESLint into CI once Node tooling is installed (`eslint`, `eslint-plugin-import`, `stylelint`, `stylelint-config-standard`).
3. Schedule a post-deploy audit to confirm dynamic states (`reqman-placeholder-feature`, `reqman-project-card--bg-*`) are safe to delete.
