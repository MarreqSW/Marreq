# Template Modularity Guide

This repository now uses shared Handlebars partials and consolidated static assets to keep the HTML templates consistent and maintainable.

## Partial Directory Structure

Reusable fragments live in `templates/partials/` and are referenced from pages with `{{> partials/<name>}}`.

| Partial | Purpose |
| ------- | ------- |
| `partials/header.html.hbs` | Dashboard header with mobile toggle. |
| `partials/sidebar.html.hbs` | Dashboard sidebar navigation and theme toggle. |
| `partials/user_menu.html.hbs` | User information/footer used by the sidebar. |
| `partials/project_card.html.hbs` | Project card used on the home page and project listings. |
| `partials/quick_action_card.html.hbs` | Quick action tiles on the dashboard. |
| `partials/metrics_card.html.hbs` | Dashboard metric cards rendered via partial blocks. |
| `partials/filters_form.html.hbs` | Reusable filter controls for requirements/tests views. |
| `partials/modals.html.hbs` | Modal dialogs for creating requirements and tests. |
| `partials/nav.html.hbs` | Main top navigation bar. |

## Static Assets

The main stylesheet entry point is `src/html/static/reqman.css`, which stitches together modular partials from the `src/html/static/css/` directory (tokens, utilities, navigation, tables, page-level bundles, etc.). Each partial owns a cohesive slice of the design system, keeping changes isolated while still loading as a single bundle in production. The single JavaScript entry point is `src/html/static/reqman.js`, which now:

- Handles theme toggling, sidebar state, and project selection.
- Provides table sorting, inline-editing, and modal helpers for requirements/tests.
- Performs client-side DELETE and POST/PATCH requests used by templates.

Both assets are referenced from `templates/layout.html.hbs` (and `templates/login.html.hbs` for the login screen).

## Adding or Updating Components

1. Create a partial in `templates/partials/`. Keep server-side bindings (e.g. `{{user.user_name}}`) unchanged unless you also update the backing context.
2. Import shared CSS rules or append scoped selectors to `reqman.css`.
3. If the component needs client logic, expose a small initializer inside `reqman.js` and guard it with a DOM query so other pages are unaffected.
4. Update the relevant page template to include your partial (`{{> partials/your_partial ...}}`).
5. Extend `tests/template_smoke.rs` with a representative context to keep the smoke render passing.
