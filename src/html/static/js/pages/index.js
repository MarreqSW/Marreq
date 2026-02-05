/**
 * Page-specific initialization modules
 * @module pages
 * 
 * Each page module exports an `init()` function that initializes
 * the page's interactive features when the DOM is ready.
 */

// Requirements pages
export { init as initRequirements } from './requirements.js';
export { init as initRequirementForm } from './requirementForm.js';
export { init as initRequirementsTree } from './requirementsTree.js';
export { init as initRequirementDetail } from './requirementDetail.js';
export { init as initSemanticSearch } from './semanticSearch.js';

// Other pages
export { init as initTests } from './tests.js';
export { init as initMatrix } from './matrix.js';
export { init as initCategories } from './categories.js';
export { init as initApplicability } from './applicability.js';
export { init as initLogs } from './logs.js';
export { init as initEntityLogs } from './entityLogs.js';
export { init as initLogAnalytics } from './logAnalytics.js';
export { init as initMapColumns } from './mapColumns.js';

// Admin pages
export { init as initAdminBackup } from './adminBackup.js';
export { init as initAdminCacheHealth } from './adminCacheHealth.js';
export { init as initAdminCacheStats } from './adminCacheStats.js';
