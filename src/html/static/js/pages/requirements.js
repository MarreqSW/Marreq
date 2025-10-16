import { initTableSort } from '../modules/sortTable.js';
import { enableInlineTextEditing, enableInlineChangeHandling } from '../modules/inlineEdit.js';
import { patchJson } from '../core/net.js';
import { bindModalForm } from '../modules/modals.js';
import { showNotification } from '../modules/notifications.js';
import { postJson } from '../core/net.js';

const numericFields = new Set([
  'req_current_status',
  'req_verification',
  'req_author',
  'req_reviewer',
  'req_category',
  'req_applicability',
]);

async function updateRequirementField(id, field, rawValue) {
  const payload = {};

  if (numericFields.has(field)) {
    const numeric = Number(rawValue);
    if (Number.isNaN(numeric)) {
      throw new Error('Invalid numeric value');
    }
    payload[field] = numeric;
  } else {
    payload[field] = rawValue;
  }

  await patchJson(`/api/v1/requirements/${id}`, payload);
}

function initRequirementTable() {
  const table = document.getElementById('requirementsTable');
  if (!table) {
    return;
  }

  initTableSort(table, {
    req_id: 0,
    req_title: 1,
    req_reference: 2,
    req_category: 3,
    req_current_status: 4,
    req_verification: 5,
    req_author: 6,
    req_reviewer: 7,
    req_creation_date: 8,
    req_deadline_date: 9,
  });

  enableInlineTextEditing(table, '.editable-field', async ({ id, field, value, revert }) => {
    try {
      await updateRequirementField(id, field, value);
      showNotification('Requirement updated successfully', 'success');
    } catch (error) {
      showNotification(error.message || 'Error updating requirement', 'error');
      revert();
    }
  });

  const handleChange = async ({ id, field, value }) => {
    try {
      await updateRequirementField(id, field, value);
      showNotification('Requirement updated successfully', 'success');
    } catch (error) {
      showNotification(error.message || 'Error updating requirement', 'error');
    }
  };

  enableInlineChangeHandling(table, '.editable-select', handleChange);
  enableInlineChangeHandling(table, '.editable-date', handleChange);
}

function initCreateRequirementModal() {
  bindModalForm({
    triggerSelector: '#addNewRequirement',
    modalSelector: '#addRequirementModal',
    formSelector: '#addRequirementForm',
    successMessage: 'Requirement added successfully',
    errorMessage: 'Error adding requirement',
    handleSubmit: async ({ data }) => {
      await postJson('/api/v1/requirements', data);
      setTimeout(() => window.location.reload(), 600);
    },
  });
}

export function init() {
  initRequirementTable();
  initCreateRequirementModal();
}
