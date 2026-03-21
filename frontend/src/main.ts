/**
 * SPA entry: reuses legacy client bundle from src/html/static (see plan: split API + SPA).
 * Set data-page before loading app.js so initPageController() runs the right module.
 */
if (!document.body.dataset.page) {
  document.body.dataset.page = 'index';
}

import '@static/css/index.css';
import '@static/marreq.css';
import '@static/js/app.js';
