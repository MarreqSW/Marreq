/**
 * Trace-builder component – add / remove parent (upstream) links.
 *
 * Uses data-role selectors to find DOM elements and c-trace-builder__*
 * BEM classes for dynamically created markup.
 * Requirement and link-type selectors use the shared c-custom-dropdown
 * component (initialised by customDropdown.js via initCustomDropdowns).
 *
 * Depends on: css/50-components/trace-builder.css
 *             css/50-components/dropdown.css
 */

import { showNotification } from './notifications.js';

/* ── Helpers ───────────────────────────────────────────────────── */

function escapeHtml(s) {
  const div = document.createElement('div');
  div.textContent = s;
  return div.innerHTML;
}

/**
 * Reset a c-custom-dropdown back to its placeholder state.
 * Clears the hidden <select> value and updates the visible trigger text.
 */
function resetDropdown(dropdown) {
  if (!dropdown) return;
  const hiddenSelect = dropdown.querySelector('select');
  const valueDisplay = dropdown.querySelector('[data-role="dropdown-value"]');
  if (hiddenSelect) {
    hiddenSelect.value = '';
  }
  if (valueDisplay) {
    const placeholder = dropdown.dataset.placeholder || `Select ${dropdown.dataset.dropdown}...`;
    valueDisplay.textContent = placeholder;
    valueDisplay.classList.add('c-custom-dropdown__value--placeholder');
  }
  // Clear any selected-state highlights on items
  dropdown.querySelectorAll('.c-custom-dropdown__item--selected').forEach((item) => {
    item.classList.remove('c-custom-dropdown__item--selected');
  });
}

/* ── Sync hidden JSON field from visible list ──────────────────── */

export function syncParentLinksFromList(form) {
  const list = form.querySelector('[data-role="parent-links-list"]');
  const hidden = form.querySelector('[data-role="parent-links-value"]') || form.querySelector('#parent_links');
  const parentIdField = form.querySelector('#parent_id');
  if (!list || !hidden) return;

  const items = list.querySelectorAll('li[data-target-id][data-link-type], .c-trace-builder__link-item[data-target-id][data-link-type]');
  const arr = Array.from(items).map((el) => ({
    target_requirement_id: parseInt(el.dataset.targetId, 10),
    link_type: el.dataset.linkType || 'DERIVES_FROM',
  }));
  hidden.value = JSON.stringify(arr);

  if (parentIdField) {
    parentIdField.value = arr.length > 0 ? String(arr[0].target_requirement_id) : '0';
  }
}

/* ── Init ──────────────────────────────────────────────────────── */

export function initTraceBuilder(form) {
  const section = form.querySelector('[data-role="upstream-trace"]');
  const list = form.querySelector('[data-role="parent-links-list"]');
  const addBlock = form.querySelector('[data-role="add-parent-link"]');
  const requirementSelect = form.querySelector('[data-role="parent-link-requirement"]');
  const linkTypeSelect = form.querySelector('[data-role="parent-link-type"]');
  const addBtn = form.querySelector('[data-role="add-parent-link-btn"]');

  // Dropdown wrappers (for resetting display after add)
  const reqDropdown = addBlock?.querySelector('[data-dropdown="parent-requirement"]');

  if (!section || !list || !addBlock || !requirementSelect || !linkTypeSelect || !addBtn) {
    return;
  }

  // Seed hidden field from any server-rendered items
  syncParentLinksFromList(form);

  // Add a parent link
  addBtn.addEventListener('click', () => {
    const reqId = requirementSelect.value;
    const linkType = linkTypeSelect.value;

    if (!reqId) {
      showNotification('Select a requirement to add as parent', 'error');
      return;
    }

    const opt = requirementSelect.options[requirementSelect.selectedIndex];
    const reference = opt?.getAttribute('data-reference') ?? '';
    const title = opt?.getAttribute('data-title') ?? '';
    const displayRef = reference || `RM-${reqId}`;

    const existing = list.querySelector(`[data-target-id="${reqId}"]`);
    if (existing) {
      showNotification('This requirement is already added as a parent', 'error');
      return;
    }

    const li = document.createElement('li');
    li.className = 'c-trace-builder__link-item';
    li.dataset.targetId = reqId;
    li.dataset.linkType = linkType;
    li.innerHTML = `
      <span class="c-trace-builder__link-badge">${escapeHtml(linkType)}</span>
      <span class="c-trace-builder__link-label">${escapeHtml(displayRef)} — ${escapeHtml(title)}</span>
      <button type="button" class="c-trace-builder__link-remove" data-role="remove-parent-link" aria-label="Remove parent link">Remove</button>
    `;

    list.appendChild(li);
    syncParentLinksFromList(form);

    // Reset the requirement dropdown to placeholder
    resetDropdown(reqDropdown);
  });

  // Delegated remove
  list.addEventListener('click', (e) => {
    const btn = e.target.closest('[data-role="remove-parent-link"]');
    if (btn) {
      btn.closest('li, .c-trace-builder__link-item')?.remove();
      syncParentLinksFromList(form);
    }
  });
}
