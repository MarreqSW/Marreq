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
    test_id: 0,
    test_name: 1,
    test_reference: 2,
    test_description: 3,
    test_status: 4,
    test_source: 5,
    test_parent: 6,
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

export function init() {
  initTestTable();
  initCreateTestModal();
}
