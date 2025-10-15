# Frontend Smoke Test

_Execute after building the Rust backend and serving the Handlebars templates._

## Common

- Clear browser cache or use an incognito session to avoid stale assets.
- Verify console: no errors or warnings during each journey.
- Monitor network tab: all API calls return `2xx` (or expected `4xx` on validation failures).

## Requirements

1. Navigate to `/p/{project}/requirements`.
2. Use filters: choose status + category, submit, verify table updates and counts.
3. Click `New Requirement`; submit minimal valid payload; confirm toast, modal closes, list reloads.
4. Inline-edit title and select fields; ensure success toast and persisted changes on reload.
5. Delete a draft requirement via card or table button; confirm removal.

## Tests

1. Visit `/p/{project}/tests`.
2. Add new test via modal; confirm creation and page refresh.
3. Inline-edit description/status; ensure toasts + persistence.
4. Delete a draft test; check success pathway.

## Matrix

1. Open `/p/{project}/matrix`.
2. Change status filter; ensure form autosubmits and results refresh.
3. Click column sorts for requirements and a test column; confirm indicators update, URLs retain filter.
4. Drag horizontal scroll thumb across table; ensure synced scroll.

## Categories

1. Navigate to `/p/{project}/categories`.
2. Use search field; verify rows hide/show, empty message toggles as expected.
3. Trigger delete confirmation; ensure cancel preserves row, confirm removes and reloads list.

## Requirements Tree

1. Visit `/p/{project}/requirements/tree`.
2. Confirm tree loads collapsed by default.
3. Expand/collapse individual nodes via chevron.
4. Use `Expand All` / `Collapse All` buttons to verify global behaviour.

## Applicability

1. Go to `/p/{project}/applicability`.
2. Delete an applicability entry; confirm refresh.

## Logs & Entity Logs

1. Visit `/logs` and `/logs/{entity_type}/{id}`.
2. Click `View Changes`; confirm diff modal shows JSON highlighting.
3. Ensure timestamps formatted in locale, badges styled per action type.

## Log Analytics

1. Navigate to `/log_analytics`.
2. Verify averages (`data-average`) show calculated numbers.
3. Trigger “Clean up old logs” button; confirm confirmation dialog and success response.

## Admin Cache

1. `/admin/cache_stats`: confirm auto-refresh (30s) and manual refresh button reloads page.
2. `/admin/cache_health`: observe metrics update every 10s; manually trigger refresh button; toggle tab visibility to ensure polling pauses/resumes.
3. `/admin/backup`: generate backup + logs export; ensure filenames include timestamp.

## Map Columns Import

1. Walk through `/map_columns`; select mappings, submit; inspect payload (network tab) for JSON field.

## Login

1. Log out and visit `/login`.
2. Toggle theme switch; confirm persists across reload (localStorage).
3. Sign in with valid credentials; ensure redirect to dashboard without console errors.
