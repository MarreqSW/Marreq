/**
 * SPA entry: reuses legacy client bundle from src/html/static (see plan: split API + SPA).
 * `data-page` is set in index.html (e.g. login for the split-stack shell).
 */
import '@static/js/theme-prefetch.js';
import '@static/css/index.css';
import '@static/marreq.css';
import '@static/js/app.js';
