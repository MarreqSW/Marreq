import { initThemeControls } from './modules/theme.js';
import { initSidebar } from './modules/sidebar.js';
import { initProjectSelector } from './modules/projectSelector.js';
import { initProjectPreview } from './modules/projectPreview.js';
import { initRequirementPreview } from './modules/requirementPreview.js';
import { initTestPreview } from './modules/testPreview.js';
import { registerDeleteAction } from './modules/deleteActions.js';
import { initStatusColorPickers } from './modules/statusColorPicker.js';

const pageControllers = {
  requirements: () => import('./pages/requirements.js'),
  tests: () => import('./pages/tests.js'),
  matrix: () => import('./pages/matrix.js'),
  'requirements-tree': () => import('./pages/requirementsTree.js'),
  categories: () => import('./pages/categories.js'),
  applicability: () => import('./pages/applicability.js'),
  verification: () => import('./pages/verification.js'),
  new_verification: () => Promise.resolve({}),
  edit_verification: () => Promise.resolve({}),
  'requirement-form': () => import('./pages/requirementForm.js'),
  'requirement-detail': () => import('./pages/requirementDetail.js'),
  baseline: () => import('./pages/baselineDetail.js'),
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
    onSuccess: (button) => {
      const projectId = button.getAttribute('data-project-id');
      window.location.href = `/p/${projectId}/requirements`;
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
    onSuccess: (button) => {
      const projectId = button.getAttribute('data-project-id');
      window.location.href = `/p/${projectId}/tests`;
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
    selector: '[data-action="delete-custom-field"]',
    getUrl: (button) => {
      const projectId = button.getAttribute('data-project-id');
      const fieldId = button.getAttribute('data-field-id');
      return `/p/${projectId}/custom_fields/delete/${fieldId}`;
    },
    getMessage: (button) => {
      const title = button.getAttribute('data-field-label') || 'this custom field';
      return `Are you sure you want to delete ${title}? This action cannot be undone.`;
    },
    onSuccess: (button) => {
      const projectId = button.getAttribute('data-project-id');
      window.location.href = `/p/${projectId}/custom_fields`;
    },
  });

  registerDeleteAction({
    selector: '[data-action="delete-applicability"]',
    getUrl: (button) => {
      const projectId = button.getAttribute('data-project-id');
      const applicabilityId = button.getAttribute('data-applicability-id');
      return `/p/${projectId}/applicability/delete/${applicabilityId}`;
    },
    getMessage: (button) => {
      const title = button.getAttribute('data-applicability-title') || 'this applicability';
      return `Are you sure you want to delete ${title}? This action cannot be undone.`;
    },
  });

  registerDeleteAction({
    selector: '[data-action="delete-requirement-status"]',
    getUrl: (button) => {
      const projectId = button.getAttribute('data-project-id');
      const statusId = button.getAttribute('data-status-id');
      return `/p/${projectId}/requirement_statuses/delete/${statusId}`;
    },
    getMessage: (button) => {
      const title = button.getAttribute('data-status-title') || 'this requirement status';
      return `Are you sure you want to delete ${title}? This action cannot be undone.`;
    },
  });

  registerDeleteAction({
    selector: '[data-action="delete-test-status"]',
    getUrl: (button) => {
      const projectId = button.getAttribute('data-project-id');
      const statusId = button.getAttribute('data-status-id');
      return `/p/${projectId}/test_statuses/delete/${statusId}`;
    },
    getMessage: (button) => {
      const title = button.getAttribute('data-status-title') || 'this test status';
      return `Are you sure you want to delete ${title}? This action cannot be undone.`;
    },
  });

  registerDeleteAction({
    selector: '[data-action="delete-verification"]',
    getUrl: (button) => {
      const projectId = button.getAttribute('data-project-id');
      const verificationId = button.getAttribute('data-verification-id');
      return `/p/${projectId}/verification/delete/${verificationId}`;
    },
    getMessage: (button) => {
      const title = button.getAttribute('data-verification-title') || 'this verification method';
      return `Are you sure you want to delete ${title}? Requirements linked to it will have that link removed.`;
    },
    onSuccess: (button) => {
      const projectId = button.getAttribute('data-project-id');
      window.location.href = `/p/${projectId}/verification`;
    },
  });

  registerDeleteAction({
    selector: '[data-action="delete-user"]',
    getUrl: (button) => {
      const userId = button.getAttribute('data-user-id');
      return `/user/${userId}/delete`;
    },
    getMessage: (button) => {
      const name = button.getAttribute('data-user-name') || 'this user';
      return `Are you sure you want to delete user "${name}"? This action cannot be undone.`;
    },
    onSuccess: () => {
      window.location.href = '/admin/users';
    },
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
initProjectPreview();
initRequirementPreview();
initTestPreview();
initGlobalDeleteHandlers();
initStatusColorPickers();
initConfirmations();
initHistoryBack();
initPageController();
