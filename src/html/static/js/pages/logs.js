import { initDiffModal } from '../modules/diffModal.js';

const ACTION_COLOR = {
  CREATE: 'success',
  UPDATE: 'primary',
  DELETE: 'danger',
  LOGIN: 'info',
  LOGOUT: 'secondary',
  EXPORT: 'warning',
  IMPORT: 'warning',
  STATUS_CHANGE: 'info',
};

function formatDate(value) {
  try {
    const date = new Date(value);
    if (Number.isNaN(date.valueOf())) {
      return value;
    }
    return date.toLocaleString();
  } catch (error) {
    return value;
  }
}

export function init() {
  document.querySelectorAll('[data-timestamp]').forEach((cell) => {
    const raw = cell.getAttribute('data-timestamp');
    if (raw) {
      cell.textContent = formatDate(raw);
    }
  });

  document.querySelectorAll('[data-action-type]').forEach((badge) => {
    const action = badge.getAttribute('data-action-type');
    const color = ACTION_COLOR[action] || 'secondary';
    badge.className = `badge bg-${color}`;
  });

  initDiffModal({
    triggerSelector: '[data-action="show-changes"]',
    modalSelector: '#changesModal',
    contentSelector: '#changesContent',
  });
}

