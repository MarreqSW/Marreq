import { initTableSort } from '../modules/sortTable.js';
import { enableInlineTextEditing, enableInlineChangeHandling } from '../modules/inlineEdit.js';
import { bindModalForm } from '../modules/modals.js';
import { showNotification } from '../modules/notifications.js';
import { postJson } from '../core/net.js';

async function updateTestField(id, field, value) {
  await postJson(`/api/v1/tests/${id}/field`, { field, value });
}

function initTestTable() {
  const table = document.getElementById('testsTable');
  if (!table) {
    return;
  }

  initTableSort(table, {
    id: 0,
    name: 1,
    reference_code: 2,
    description: 3,
    status_id: 4,
    source: 5,
    parent_id: 6,
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
      await postJson('/api/v1/tests', data);
      setTimeout(() => window.location.reload(), 600);
    },
  });
}

function initFilterToggle() {
  const toggleButton = document.querySelector('.js-toggle-filters');
  const filterBody = document.querySelector('.reqman-tests-page__filters-body');
  
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
    
    switch(viewName) {
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
    const savedView = localStorage.getItem(VIEW_KEY) || 'table';
    switchView(savedView);
  } catch (e) {
    switchView('table');
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
      const response = await fetch(`/p/${projectId}/tests/delete/${testId}`, {
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
    form.querySelectorAll('input[type="search"], input[type="text"]').forEach(input => {
      input.value = '';
    });
    form.querySelectorAll('select').forEach(select => {
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
  initDeleteButtons();
  initRowDetails();
  initFilterClear();
}
