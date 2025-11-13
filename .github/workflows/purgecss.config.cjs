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
      /^status-/,
      // Status badge variants (dynamically added)
      /^reqman-requirements-status-badge--/,
      // Editor states
      /^c-editor-dropzone--/,
      /^c-editor-dropzone__/,
      // Table states
      /^c-table-editable/,
      /^c-table-sort-trigger/,
      // Tree component (used via JS)
      /^c-tree__requirement-link/,
      /^c-tree__indicator/,
      // Form components
      /^c-form-select--/,
      /^c-custom-dropdown__item--/,
      /^c-custom-dropdown__value--/,
      // Dashboard & metrics (dynamically rendered)
      /^reqman-action-card/,
      /^reqman-project-card/,
      // Create form fields
      /^c-create-field/,
      // Utility classes
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
      /reqman-action-card/,
      /reqman-project-card/,
      /c-editor-dropzone/,
      /c-table-editable/,
      /c-table-sort-trigger/,
      /c-tree__/,
      /c-custom-dropdown/,
      /c-create-field/,
      /c-matrix-card/,
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
