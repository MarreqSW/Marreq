/**
 * Reusable UI modules
 * @module modules
 * 
 * Public API for reusable UI components and utilities used across pages.
 */

// Tree component
export { initTreeControls, searchTree, filterTree } from './tree.js';

// Notifications
export { showNotification } from './notifications.js';

// Modals
export { initDiffModal } from './diffModal.js';

// Form validation
export { initRequirementReferenceValidation } from './referenceValidator.js';

// Theme
export { initTheme } from './theme.js';

// Note: Other modules (deleteActions, inlineEdit, modals, projectSelector, 
// scrollIndicator, sidebar, sortTable, tableFilter) are available but 
// considered internal/page-specific utilities.
