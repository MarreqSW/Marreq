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
  tests: () => import('./pages/verifications.js'),
  verifications: () => import('./pages/verifications.js'),
  matrix: () => import('./pages/matrix.js'),
  'requirements-tree': () => import('./pages/requirementsTree.js'),
  categories: () => import('./pages/categories.js'),
  applicability: () => import('./pages/applicability.js'),
  groups: () => import('./pages/groups.js'),
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

function getProjectRouteKey(element) {
  return (
    element.getAttribute('data-project-slug') ||
    element.getAttribute('data-project-id') ||
    ''
  );
}

function initGlobalDeleteHandlers() {
  registerDeleteAction({
    selector: '[data-action="delete-requirement"]',
    getUrl: (button) => {
      const projectSlug = getProjectRouteKey(button);
      const requirementId = button.getAttribute('data-requirement-id');
      return `/${projectSlug}/requirements/delete/${requirementId}`;
    },
    getMessage: (button) => {
      const title = button.getAttribute('data-requirement-title') || 'Requirement';
      return `Are you sure you want to delete requirement "${title}"? This action cannot be undone.`;
    },
    onSuccess: (button) => {
      const projectSlug = getProjectRouteKey(button);
      window.location.href = `/${projectSlug}/requirements`;
    },
  });

  registerDeleteAction({
    selector: '[data-action="delete-test"]',
    getUrl: (button) => {
      const projectSlug = getProjectRouteKey(button);
      const testId = button.getAttribute('data-test-id');
      return `/${projectSlug}/verifications/delete/${testId}`;
    },
    getMessage: (button) => {
      const name = button.getAttribute('data-test-name') || 'Test';
      return `Are you sure you want to delete test "${name}"? This action cannot be undone.`;
    },
    onSuccess: (button) => {
      const projectSlug = getProjectRouteKey(button);
      window.location.href = `/${projectSlug}/verifications`;
    },
  });

  registerDeleteAction({
    selector: '[data-action="delete-project"]',
    getUrl: (button) => {
      const projectSlug = getProjectRouteKey(button);
      return `/${projectSlug}/delete`;
    },
    getMessage: (button) => {
      const name = button.getAttribute('data-project-name') || 'this project';
      return `Are you sure you want to delete project "${name}"? This action cannot be undone.`;
    },
  });

  registerDeleteAction({
    selector: '[data-action="delete-category"]',
    getUrl: (button) => {
      const projectSlug = getProjectRouteKey(button);
      const categoryId = button.getAttribute('data-category-id');
      return `/${projectSlug}/categories/delete/${categoryId}`;
    },
    getMessage: (button) => {
      const title = button.getAttribute('data-category-title') || 'this category';
      return `Are you sure you want to delete ${title}? This action cannot be undone.`;
    },
  });

  registerDeleteAction({
    selector: '[data-action="delete-custom-field"]',
    getUrl: (button) => {
      const projectSlug = getProjectRouteKey(button);
      const fieldId = button.getAttribute('data-field-id');
      return `/${projectSlug}/custom_fields/delete/${fieldId}`;
    },
    getMessage: (button) => {
      const title = button.getAttribute('data-field-label') || 'this custom field';
      return `Are you sure you want to delete ${title}? This action cannot be undone.`;
    },
    onSuccess: (button) => {
      const projectSlug = getProjectRouteKey(button);
      window.location.href = `/${projectSlug}/custom_fields`;
    },
  });

  registerDeleteAction({
    selector: '[data-action="delete-applicability"]',
    getUrl: (button) => {
      const projectSlug = getProjectRouteKey(button);
      const applicabilityId = button.getAttribute('data-applicability-id');
      return `/${projectSlug}/applicability/delete/${applicabilityId}`;
    },
    getMessage: (button) => {
      const title = button.getAttribute('data-applicability-title') || 'this applicability';
      return `Are you sure you want to delete ${title}? This action cannot be undone.`;
    },
  });

  registerDeleteAction({
    selector: '[data-action="delete-requirement-status"]',
    getUrl: (button) => {
      const projectSlug = getProjectRouteKey(button);
      const statusId = button.getAttribute('data-status-id');
      return `/${projectSlug}/requirement_statuses/delete/${statusId}`;
    },
    getMessage: (button) => {
      const title = button.getAttribute('data-status-title') || 'this requirement status';
      return `Are you sure you want to delete ${title}? This action cannot be undone.`;
    },
  });

  registerDeleteAction({
    selector: '[data-action="delete-verification-status"]',
    getUrl: (button) => {
      const projectSlug = getProjectRouteKey(button);
      const statusId = button.getAttribute('data-status-id');
      return `/${projectSlug}/verification_statuses/delete/${statusId}`;
    },
    getMessage: (button) => {
      const title = button.getAttribute('data-status-title') || 'this verification status';
      return `Are you sure you want to delete ${title}? This action cannot be undone.`;
    },
  });

  registerDeleteAction({
    selector: '[data-action="delete-verification"]',
    getUrl: (button) => {
      const projectSlug = getProjectRouteKey(button);
      const verificationId = button.getAttribute('data-verification-id');
      return `/${projectSlug}/verification/delete/${verificationId}`;
    },
    getMessage: (button) => {
      const title = button.getAttribute('data-verification-title') || 'this verification method';
      return `Are you sure you want to delete ${title}? Requirements linked to it will have that link removed.`;
    },
    onSuccess: (button) => {
      const projectSlug = getProjectRouteKey(button);
      window.location.href = `/${projectSlug}/verification`;
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
