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
      // Editor states
      /^c-editor-dropzone--/,
      /^c-editor-status__menu/,
      /^c-editor-linked__/,
      /^c-editor-field__error/,
      /^c-editor-preview__empty/,
      /^c-editor-dropzone__/,
      // Table states
      /^c-table-notification--/,
      // Generic utility classes that might be used dynamically
      /^text-/,
      /^fw-/,
      /^btn-/,
    ],
    deep: [
      // Keep all variations of these components
      /reqman-requirements-status-badge/,
      /c-editor-/,
      /c-table-notification/,
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
