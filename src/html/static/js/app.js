import { initThemeControls } from './modules/theme.js';
import { initSidebar } from './modules/sidebar.js';
import { initProjectSelector } from './modules/projectSelector.js';
import { registerDeleteAction } from './modules/deleteActions.js';

const pageControllers = {
  requirements: () => import('./pages/requirements.js'),
  tests: () => import('./pages/tests.js'),
  matrix: () => import('./pages/matrix.js'),
  'requirements-tree': () => import('./pages/requirementsTree.js'),
  categories: () => import('./pages/categories.js'),
  'requirement-form': () => import('./pages/requirementForm.js'),
  'log-analytics': () => import('./pages/logAnalytics.js'),
  logs: () => import('./pages/logs.js'),
  'entity-logs': () => import('./pages/entityLogs.js'),
  'map-columns': () => import('./pages/mapColumns.js'),
  'admin-cache-stats': () => import('./pages/adminCacheStats.js'),
  'admin-cache-health': () => import('./pages/adminCacheHealth.js'),
  'admin-backup': () => import('./pages/adminBackup.js'),
};

function initGlobalDeleteHandlers() {
  registerDeleteAction({
    selector: '[data-action="delete-requirement"]',
    getUrl: (button) => {
      const projectId = button.getAttribute('data-project-id');
      const requirementId = button.getAttribute('data-requirement-id');
      return `/p/${projectId}/requirements/delete/${requirementId}`;
    },
    getMessage: (button) => {
      const title = button.getAttribute('data-requirement-title') || 'Requirement';
      return `Are you sure you want to delete requirement "${title}"? This action cannot be undone.`;
    },
  });

  registerDeleteAction({
    selector: '[data-action="delete-test"]',
    getUrl: (button) => {
      const projectId = button.getAttribute('data-project-id');
      const testId = button.getAttribute('data-test-id');
      return `/p/${projectId}/tests/delete/${testId}`;
    },
    getMessage: (button) => {
      const name = button.getAttribute('data-test-name') || 'Test';
      return `Are you sure you want to delete test "${name}"? This action cannot be undone.`;
    },
  });

  registerDeleteAction({
    selector: '[data-action="delete-project"]',
    getUrl: (button) => {
      const projectId = button.getAttribute('data-project-id');
      return `/p/${projectId}/delete`;
    },
    getMessage: (button) => {
      const name = button.getAttribute('data-project-name') || 'this project';
      return `Are you sure you want to delete project "${name}"? This action cannot be undone.`;
    },
  });

  registerDeleteAction({
    selector: '[data-action="delete-category"]',
    getUrl: (button) => {
      const projectId = button.getAttribute('data-project-id');
      const categoryId = button.getAttribute('data-category-id');
      return `/p/${projectId}/categories/delete/${categoryId}`;
    },
    getMessage: (button) => {
      const title = button.getAttribute('data-category-title') || 'this category';
      return `Are you sure you want to delete ${title}? This action cannot be undone.`;
    },
  });

  registerDeleteAction({
    selector: '[data-action="delete-applicability"]',
    getUrl: (button) => {
      const projectId = button.getAttribute('data-project-id');
      const applicabilityId = button.getAttribute('data-applicability-id');
      return `/p/${projectId}/applicability/delete/${applicabilityId}`;
    },
    getMessage: () => 'Are you sure you want to delete this applicability? This action cannot be undone.',
  });
}

function initConfirmations() {
  document.addEventListener('click', (event) => {
    const trigger = event.target.closest('[data-confirm]');
    if (!trigger) {
      return;
    }

    const message = trigger.getAttribute('data-confirm') || 'Are you sure?';
    if (!window.confirm(message)) {
      event.preventDefault();
      event.stopImmediatePropagation();
    }
  });
}

function initHistoryBack() {
  document.addEventListener('click', (event) => {
    const trigger = event.target.closest('[data-action="history-back"]');
    if (!trigger) {
      return;
    }

    event.preventDefault();
    window.history.back();
  });
}

function initPageController() {
  const page = document.body.dataset.page;
  if (!page || !pageControllers[page]) {
    return;
  }

  pageControllers[page]()
    .then((module) => {
      if (module?.init) {
        module.init();
      }
    })
    .catch((error) => {
      console.error(`Failed to load controller for page "${page}":`, error);
    });
}

initThemeControls();
initSidebar();
initProjectSelector();
initGlobalDeleteHandlers();
initConfirmations();
initHistoryBack();
initPageController();
