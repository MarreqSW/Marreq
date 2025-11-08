module.exports = {
  content: [
    'templates/**/*.hbs',
    'templates/**/*.html.hbs',
    'src/html/static/**/*.js',
    'tests/**/*.js',
  ],
  css: [
    'src/html/static/**/*.css',
  ],
  // Safelist patterns that should never be removed
  safelist: {
    standard: [
      // Dynamic classes that might be added via JavaScript
      /^js-/,
      /^is-/,
      /^has-/,
      /^status-/,
      // Status badge variants (dynamically added)
      /^reqman-requirements-status-badge--/,
      /^reqman-badge--bg-/,
      /^reqman-project-card--bg-/,
      // Editor states
      /^c-editor-dropzone--/,
      /^c-editor-status__menu/,
      /^c-editor-linked__/,
      /^c-editor-field__error/,
      /^c-editor-preview__empty/,
      /^c-editor-dropzone__/,
      // Table states
      /^c-table-notification--/,
      /^c-table-editable/,
      /^c-table__cell--/,
      /^c-table-view/,
      /^c-table-sort-trigger/,
      // Tree component (used via JS)
      /^c-tree__requirement-link/,
      /^c-tree__indicator/,
      /^c-tree__empty-state/,
      /^c-tree__breadcrumb/,
      // Form components
      /^c-filter-section/,
      /^c-form-select--/,
      // Navigation
      /^c-navbar__project-/,
      /^c-navbar__link--active/,
      // Dashboard & metrics (dynamically rendered)
      /^reqman-metric/,
      /^reqman-action-card/,
      /^reqman-project-card/,
      // Components (conditional/dynamic)
      /^c-spinner/,
      /^c-linked-tests/,
      /^c-test-info/,
      /^c-portal-/,
      /^c-requirement-section/,
      /^c-requirement-card__reference-tag/,
      /^c-card__subtitle/,
      // Create form fields (on specific page)
      /^c-create-field-grid/,
      /^c-create-field__meta/,
      // Utility classes
      /^u-list/,
      /^u-text-/,
      /^o-grid--/,
      // Generic utility classes that might be used dynamically
      /^text-/,
      /^fw-/,
      /^btn-/,
    ],
    deep: [
      // Keep all variations of these components
      /reqman-requirements-status-badge/,
      /reqman-badge/,
      /reqman-project-card/,
      /reqman-metric/,
      /c-editor-/,
      /c-table-notification/,
      /c-table-editable/,
      /c-table-view/,
      /c-tree__/,
      /c-filter-section/,
      /c-navbar__project/,
      /c-linked-/,
      /c-test-info/,
      /c-portal-/,
      /c-requirement-section/,
      /c-create-field/,
    ],
    greedy: [
      // Status variants
      /--draft$/,
      /--proposal$/,
      /--accepted$/,
      /--approved$/,
      /--finished$/,
      /--passed$/,
      /--rejected$/,
      /--failed$/,
      /--cancelled$/,
      /--default$/,
      /--success$/,
      /--warning$/,
      /--error$/,
      /--bg-success$/,
      /--bg-warning$/,
      /--bg-secondary$/,
      /--bg-light$/,
      /--bg-white$/,
      // Interaction states
      /:hover$/,
      /:focus$/,
      /:focus-within$/,
      /:focus-visible$/,
      /:last-child$/,
      /:checked$/,
      // Dark theme variants
      /^\[data-theme='dark'\]/,
    ],
  },
  // Don't remove CSS that might be used dynamically
  defaultExtractor: (content) => content.match(/[\w-/:%.@]+(?<!:)/g) || [],
  // Report which selectors were removed
  rejected: true,
  rejectedCss: true,
  variables: true,
  keyframes: true,
  fontFace: true,
};
