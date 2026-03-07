import { initTableSort } from '../modules/sortTable.js';
import { enableInlineTextEditing, enableInlineChangeHandling } from '../modules/inlineEdit.js';
import { bindModalForm } from '../modules/modals.js';
import { showNotification } from '../modules/notifications.js';
import { postJson } from '../core/net.js';

function getTestsInlineEditConfig() {
  const script = document.getElementById('testsInlineEditConfig');
  if (!script?.textContent) return { statuses: [], categories: [], verifications: [] };
  try {
    return JSON.parse(script.textContent.trim());
  } catch {
    return { statuses: [], categories: [], verifications: [] };
  }
}

/** Maps status label to CSS variant (must match server status_variant). */
function testStatusVariant(statusLabel) {
  if (!statusLabel) return 'default';
  const s = String(statusLabel).toLowerCase();
  if (s.includes('pass')) return 'passed';
  if (s.includes('fail')) return 'failed';
  if (s.includes('pending')) return 'proposal';
  if (s.includes('progress')) return 'draft';
  return 'default';
}

/** Update data-test-preview-status on the row's title link after inline status edit */
function updateTestPreviewInRow(row, displayText) {
  const titleLink = row.querySelector('a.marreq-requirements-title[data-test-preview]');
  if (titleLink && displayText != null) {
    titleLink.setAttribute('data-test-preview-status', displayText);
  }
}

/** Update the matching card's status badge when inline edit succeeds (keeps card/table in sync) */
function updateCardStatusBadge(testId, displayText, variant, tagColor = null) {
  const card = document.querySelector(`#cardView .marreq-requirement-card[data-test-id="${testId}"]`);
  if (!card) return;
  const badge = card.querySelector('.marreq-requirement-card__header .marreq-requirements-status-badge');
  if (badge) {
    badge.textContent = displayText;
    badge.className = `marreq-requirements-status-badge marreq-requirements-status-badge--${variant}`;
    badge.dataset.status = displayText;
    badge.dataset.statusVariant = variant;
    if (tagColor) {
      badge.style.backgroundColor = tagColor;
      badge.style.color = '#fff';
      badge.style.borderColor = tagColor;
    } else {
      badge.style.backgroundColor = '';
      badge.style.color = '';
      badge.style.borderColor = '';
    }
  }
  const cardTitleLink = card.querySelector('a.marreq-requirement-card__title-link[data-test-preview]');
  if (cardTitleLink && displayText != null) {
    cardTitleLink.setAttribute('data-test-preview-status', displayText);
  }
}

/** Update any parent links that point to this test (so their hover card shows the updated status). */
function updateParentLinkPreviewsForTest(testId, displayText) {
  const links = document.querySelectorAll(
    `[data-test-preview][data-test-preview-id="${testId}"]`
  );
  links.forEach((link) => {
    if (displayText != null) {
      link.setAttribute('data-test-preview-status', displayText);
    }
  });
}

/**
 * Open inline edit for test status (same pattern as requirements openInlineEdit).
 * Uses postJson from core/net.js so errors and success are handled like requirements.
 */
function openInlineEditForTest(cell, row, config) {
  const testId = parseInt(row.dataset.testId, 10);
  if (!testId) return;
  const projectId = config.projectId;
  if (!projectId) return;
  const displayEl = cell.querySelector('.marreq-requirements-cell__display');
  if (!displayEl || cell.querySelector('.marreq-inline-edit-select')) return;

  const select = document.createElement('select');
  select.className = 'marreq-inline-edit-select';
  select.setAttribute('aria-label', 'Change status');

  const currentId = parseInt(row.dataset.statusId, 10) || 0;
  (config.statuses || []).forEach((s) => {
    const sid = typeof s.id === 'number' ? s.id : parseInt(s.id, 10);
    select.appendChild(new Option(s.title, String(sid), false, sid === currentId));
  });

  const initialValue = select.value;
  let applied = false;

  const apply = async () => {
    if (applied) return;
    const v = parseInt(select.value, 10);
    if (Number.isNaN(v)) return;
    const s = (config.statuses || []).find((x) => (typeof x.id === 'number' ? x.id : parseInt(x.id, 10)) === v);
    const displayText = s ? s.title : '—';
    applied = true;
    if (select.parentNode) select.remove();
    displayEl.hidden = false;
    try {
      await postJson(`/p/${projectId}/verifications/update-status/${testId}`, { status_id: v });
      const variant = testStatusVariant(displayText);
      const tagColor = s?.tag_color || null;
      row.dataset.statusId = String(v);
      row.dataset.statusLabel = displayText;
      displayEl.textContent = displayText;
      displayEl.dataset.status = displayText;
      displayEl.dataset.statusId = String(v);
      displayEl.dataset.statusVariant = variant;
      displayEl.className = `marreq-requirements-status-badge marreq-requirements-status-badge--${variant} marreq-requirements-cell__display`;
      if (tagColor) {
        displayEl.style.backgroundColor = tagColor;
        displayEl.style.color = '#fff';
        displayEl.style.borderColor = tagColor;
      } else {
        displayEl.style.backgroundColor = '';
        displayEl.style.color = '';
        displayEl.style.borderColor = '';
      }
      updateTestPreviewInRow(row, displayText);
      updateCardStatusBadge(testId, displayText, variant, tagColor);
      updateParentLinkPreviewsForTest(testId, displayText);
      showNotification('Status updated successfully', 'success');
    } catch (err) {
      applied = false;
      const status = err?.response?.status;
      const msg = err?.message || 'Update failed';
      const detail = status ? ` (${status})` : '';
      showNotification(msg + detail, 'error');
      console.error('Test status update failed:', err?.payload || err);
      window.location.reload();
    }
  };

  select.addEventListener('change', () => apply());
  select.addEventListener('blur', () => {
    if (applied) return;
    if (select.value !== initialValue) apply();
    else {
      if (select.parentNode) select.remove();
      displayEl.hidden = false;
    }
  });
  document.addEventListener('keydown', function esc(e) {
    if (e.key === 'Escape') {
      select.remove();
      displayEl.hidden = false;
      document.removeEventListener('keydown', esc);
    }
  });

  displayEl.hidden = true;
  cell.appendChild(select);
  select.focus();
}

/** Init inline status edit on the tests table (same pattern as requirements initInlineEdit). */
function initInlineStatusEdit() {
  const table = document.getElementById('testsTable');
  if (!table) return;
  const config = getTestsInlineEditConfig();
  if (!config.statuses?.length) return;

  const pageEl = document.querySelector('.marreq-requirements-page[data-project-id]');
  const projectId = pageEl?.getAttribute('data-project-id');
  if (!projectId) return;
  const configWithProject = { ...config, projectId };

  table.addEventListener('click', (e) => {
    if (e.target.closest('.marreq-inline-edit-select')) return;
    const cell = e.target.closest('[data-inline-edit="status"]');
    if (!cell || !table.contains(cell)) return;
    e.preventDefault();
    e.stopPropagation();
    const row = cell.closest('tr');
    if (!row || !row.classList.contains('marreq-requirements-row')) return;
    openInlineEditForTest(cell, row, configWithProject);
  });
}

function initTestTable() {
  const table = document.getElementById('testsTable');
  if (!table) {
    return;
  }

  initTableSort(table, {
    reference_code: 0,
    name: 1,
    status: 2,
    verification_type: 3,
    source: 4,
    parent: 5,
  });

  enableInlineTextEditing(table, '.editable-field', async ({ id, field, value, revert }) => {
    try {
      await updateTestField(id, field, value);
      showNotification('Test updated successfully', 'success');
    } catch (error) {
      showNotification(error.message || 'Error updating test', 'error');
      revert();
    }
  });

  const handleChange = async ({ id, field, value }) => {
    try {
      await updateTestField(id, field, value);
      showNotification('Test updated successfully', 'success');
    } catch (error) {
      showNotification(error.message || 'Error updating test', 'error');
    }
  };

  enableInlineChangeHandling(table, '.editable-select', handleChange);
}

function initCreateTestModal() {
  bindModalForm({
    triggerSelector: '#addNewTest',
    modalSelector: '#addTestModal',
    formSelector: '#addTestForm',
    successMessage: 'Test added successfully',
    errorMessage: 'Error adding test',
    handleSubmit: async ({ data }) => {
      await postJson('/api/tests', data);
      setTimeout(() => window.location.reload(), 600);
    },
  });
}

function initFilterToggle() {
  const toggleButton = document.querySelector('.js-toggle-filters');
  const filterBody = document.querySelector('.marreq-tests-page__filters-body');

  if (!toggleButton || !filterBody) {
    return;
  }

  // Load saved state from localStorage
  const isCollapsed = localStorage.getItem('testsFiltersCollapsed') === 'true';
  if (isCollapsed) {
    filterBody.style.display = 'none';
    toggleButton.setAttribute('aria-expanded', 'false');
  } else {
    filterBody.style.display = 'block';
    toggleButton.setAttribute('aria-expanded', 'true');
  }

  toggleButton.addEventListener('click', () => {
    const isCurrentlyVisible = filterBody.style.display !== 'none';
    filterBody.style.display = isCurrentlyVisible ? 'none' : 'block';
    toggleButton.setAttribute('aria-expanded', !isCurrentlyVisible);

    // Save state to localStorage
    localStorage.setItem('testsFiltersCollapsed', isCurrentlyVisible);
  });
}

function initViewSwitcher() {
  const VIEW_KEY = 'tests_view_preference';
  const DEFAULT_VIEW = 'table';
  const VALID_VIEWS = ['table', 'card'];

  const cardBtn = document.getElementById('cardViewBtn');
  const tableBtn = document.getElementById('tableViewBtn');

  const cardView = document.getElementById('cardView');
  const tableView = document.getElementById('tableView');

  if (!cardBtn || !tableBtn || !cardView || !tableView) {
    return;
  }

  function switchView(viewName) {
    cardView.style.display = 'none';
    tableView.style.display = 'none';

    cardBtn.classList.remove('active');
    tableBtn.classList.remove('active');

    switch (viewName) {
      case 'card':
        cardView.style.display = 'block';
        cardBtn.classList.add('active');
        break;
      case 'table':
      default:
        tableView.style.display = 'block';
        tableBtn.classList.add('active');
        break;
    }

    try {
      localStorage.setItem(VIEW_KEY, viewName);
    } catch (e) {
      console.warn('Could not save view preference:', e);
    }
  }

  cardBtn.addEventListener('click', () => switchView('card'));
  tableBtn.addEventListener('click', () => switchView('table'));

  try {
    const saved = localStorage.getItem(VIEW_KEY);
    const view = saved && VALID_VIEWS.includes(saved) ? saved : DEFAULT_VIEW;
    switchView(view);
  } catch (e) {
    switchView(DEFAULT_VIEW);
  }
}

function initDeleteButtons() {
  document.addEventListener('click', async (e) => {
    const deleteBtn = e.target.closest('[data-action="delete-test"]');
    if (!deleteBtn) return;

    e.preventDefault();

    const testId = deleteBtn.dataset.testId;
    const testName = deleteBtn.dataset.testName;
    const projectId = deleteBtn.dataset.projectId;

    if (!confirm(`Are you sure you want to delete test "${testName}"?`)) {
      return;
    }

    try {
      const response = await fetch(`/p/${projectId}/verifications/delete/${testId}`, {
        method: 'DELETE',
        headers: {
          'Content-Type': 'application/json',
        },
      });

      if (response.ok) {
        showNotification('Test deleted successfully', 'success');
        setTimeout(() => window.location.reload(), 600);
      } else {
        showNotification('Error deleting test', 'error');
      }
    } catch (error) {
      showNotification('Error deleting test', 'error');
    }
  });
}

function initRowDetails() {
  document.addEventListener('click', (e) => {
    const toggle = e.target.closest('[data-action="toggle-row-details"]');
    if (!toggle) return;

    e.preventDefault();

    const detailsId = toggle.getAttribute('aria-controls');
    const detailsRow = document.getElementById(detailsId);

    if (!detailsRow) return;

    const isExpanded = toggle.getAttribute('aria-expanded') === 'true';

    if (isExpanded) {
      detailsRow.hidden = true;
      toggle.setAttribute('aria-expanded', 'false');
    } else {
      detailsRow.hidden = false;
      toggle.setAttribute('aria-expanded', 'true');
    }
  });
}

function initFilterClear() {
  const clearButton = document.querySelector('[data-action="clear-filters"]');
  if (!clearButton) return;

  clearButton.addEventListener('click', (e) => {
    e.preventDefault();
    const form = clearButton.closest('form');
    if (!form) return;

    // Clear all inputs and selects
    form.querySelectorAll('input[type="search"], input[type="text"]').forEach((input) => {
      input.value = '';
    });
    form.querySelectorAll('select').forEach((select) => {
      select.selectedIndex = 0;
    });

    // Submit the form to reload with no filters
    form.submit();
  });
}

export function init() {
  initTestTable();
  initCreateTestModal();
  initFilterToggle();
  initViewSwitcher();
  initInlineStatusEdit();
  initDeleteButtons();
  initRowDetails();
  initFilterClear();
}
