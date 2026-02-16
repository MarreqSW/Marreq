import { jsonFetch, postJson, patchJson } from '../core/net.js';
import { showNotification } from '../modules/notifications.js';
import { searchTree, filterTree, initTreeControls } from '../modules/tree.js';
import { init as initSemanticSearch } from './semanticSearch.js';

const state = {
  rows: [],
  cards: [],
  treeNodes: [],
  treeNodesMap: new Map(), // Cache for faster lookups
  sortKey: null,
  sortOrder: 'asc',
  searchTerm: '',
  statusMap: new Map(),
  noResultsBanner: null,
  treeRoot: null,
  currentFilters: {
    status: null,
    verification: null,
    category: null,
  },
};

const SORTERS = {
  key: (a, b) => {
    if (a.keyNumeric !== b.keyNumeric) {
      return a.keyNumeric - b.keyNumeric;
    }
    return a.keyValue.localeCompare(b.keyValue);
  },
  title: (a, b) => a.titleValue.localeCompare(b.titleValue),
  category: (a, b) => a.categoryValue.localeCompare(b.categoryValue),
  parent: (a, b) => a.parentValue.localeCompare(b.parentValue),
  status: (a, b) => a.statusValue.localeCompare(b.statusValue),
  verification: (a, b) => a.verificationValue.localeCompare(b.verificationValue),
  updated: (a, b) => a.updatedValue - b.updatedValue,
  author: (a, b) => a.authorValue.localeCompare(b.authorValue),
};

function normalize(text) {
  return (text || '').trim().toLowerCase();
}

function parseStatusDefinitions() {
  const script = document.getElementById('requirementsStatusDefinitions');
  if (!script) {
    return new Map();
  }

  try {
    const raw = script.textContent || '[]';
    const parsed = JSON.parse(raw);
    return new Map(
      parsed.map((item) => [
        normalize(item.title),
        {
          title: item.title,
          description: item.description,
          shortName: item.short_name,
          id: item.id,
        },
      ]),
    );
  } catch (error) {
    console.error('Failed to parse status definitions', error);
    return new Map();
  }
}

function statusVariant(statusLabel) {
  const label = normalize(statusLabel);
  if (!label) return 'default';
  if (label.includes('draft')) return 'draft';
  if (label.includes('proposal') || label.includes('review')) return 'proposal';
  if (
    label.includes('accept') ||
    label.includes('approve') ||
    label.includes('finish') ||
    label.includes('pass') ||
    label.includes('complete')
  ) {
    return 'accepted';
  }
  if (label.includes('reject') || label.includes('fail') || label.includes('cancel')) {
    return 'rejected';
  }
  return 'default';
}

function decorateStatusBadges() {
  const config = getInlineEditConfig();
  const statusIdToTagColor = new Map(
    (config.statuses || []).map((s) => [s.id, s.tag_color]).filter(([, c]) => c)
  );
  const statusLabelToTagColor = new Map(
    (config.statuses || []).map((s) => [normalize(s.title), s.tag_color]).filter(([, c]) => c)
  );
  const badges = document.querySelectorAll('.reqman-requirements-status-badge');
  badges.forEach((badge) => {
    const label = badge.dataset.status || badge.textContent;
    const definition = state.statusMap.get(normalize(label));
    if (definition?.description) {
      badge.title = `${definition.title} — ${definition.description}`;
    } else if (label) {
      badge.title = label;
    }

    const variant = statusVariant(label);
    badge.classList.add(`reqman-requirements-status-badge--${variant}`);

    const row = badge.closest('tr') || badge.closest('.reqman-requirement-card') || badge.closest('.c-tree__node');
    const statusId = row ? parseInt(row.dataset.statusId, 10) : NaN;
    const tagColor =
      (!Number.isNaN(statusId) && statusIdToTagColor.get(statusId)) ||
      (label && statusLabelToTagColor.get(normalize(label)));
    if (tagColor) {
      badge.style.backgroundColor = tagColor;
      badge.style.color = '#fff';
      badge.style.borderColor = tagColor;
    } else {
      badge.style.backgroundColor = '';
      badge.style.color = '';
      badge.style.borderColor = '';
    }
  });
}

function extractNumber(value) {
  if (!value) return Number.POSITIVE_INFINITY;
  const match = value.match(/(\d+)/);
  if (!match) return Number.POSITIVE_INFINITY;
  const parsed = Number.parseInt(match[1], 10);
  return Number.isNaN(parsed) ? Number.POSITIVE_INFINITY : parsed;
}

function parseDateValue(value) {
  if (!value) return 0;
  const match = value.match(
    /(?<day>\d{2})-(?<month>\d{2})-(?<year>\d{4})\s+(?<hour>\d{2}):(?<minute>\d{2}):(?<second>\d{2})/,
  );
  if (match?.groups) {
    const { day, month, year, hour, minute, second } = match.groups;
    return new Date(
      Number(year),
      Number(month) - 1,
      Number(day),
      Number(hour),
      Number(minute),
      Number(second),
    ).getTime();
  }

  const timestamp = Date.parse(value);
  return Number.isNaN(timestamp) ? 0 : timestamp;
}

function textFrom(root, selector) {
  const node = root.querySelector(selector);
  return node ? node.textContent.trim() : '';
}

function collectRows(table) {
  const entries = [];
  table.querySelectorAll('.reqman-requirements-row').forEach((row) => {
    const requirementId = row.dataset.requirementId;
    const detail = table.querySelector(
      `.reqman-requirements-row__details[data-details-for="${requirementId}"]`,
    );

    const keyText = textFrom(row, '.reqman-requirements-key__value');
    const titleText = textFrom(row, '.reqman-requirements-title');
    const categoryText = textFrom(row, '.reqman-requirements-row__cell--category');
    const parentText = textFrom(row, '.reqman-requirements-row__cell--parent');
    const statusText = (row.dataset.statusLabel || '').trim();
    const verificationText = textFrom(row, '.reqman-requirements-row__cell--verification');
    const updatedNode = row.querySelector('.reqman-requirements-row__cell--updated time');
    const updatedDisplay = updatedNode ? updatedNode.textContent.trim() : '';
    const updatedValue = parseDateValue(
      (updatedNode && updatedNode.getAttribute('datetime')) || updatedDisplay,
    );
    const authorText = textFrom(row, '.reqman-requirements-row__cell--author');
    const detailText = detail ? detail.textContent.trim() : '';

    const searchText = [
      keyText,
      titleText,
      categoryText,
      parentText,
      statusText,
      verificationText,
      updatedDisplay,
      authorText,
      detailText,
    ]
      .join(' ')
      .replace(/\s+/g, ' ')
      .toLowerCase();

    entries.push({
      id: requirementId,
      row,
      detail,
      keyValue: keyText.toLowerCase(),
      keyNumeric: extractNumber(keyText),
      titleValue: titleText.toLowerCase(),
      categoryValue: categoryText.toLowerCase(),
      parentValue: parentText.toLowerCase(),
      statusValue: statusText.toLowerCase(),
      verificationValue: verificationText.toLowerCase(),
      updatedValue,
      authorValue: authorText.toLowerCase(),
      searchText,
      visible: true,
    });
  });

  state.rows = entries;
}

function collectCards(container) {
  const entries = [];
  container.querySelectorAll('.reqman-requirement-card').forEach((card) => {
    const requirementId = card.dataset.requirementId;

    const keyText = textFrom(card, '.reqman-requirement-card__reference-text');
    const titleText = textFrom(card, '.reqman-requirement-card__title');
    const statusText = (card.dataset.statusLabel || '').trim();
    const verificationText = card.dataset.verification || '';
    const categoryText = card.dataset.category || '';
    const descriptionText = textFrom(card, '.reqman-requirement-card__description');
    const authorText = textFrom(card, '.reqman-requirement-card__author');
    const dateText = textFrom(card, '.reqman-requirement-card__date');

    const searchText = [
      keyText,
      titleText,
      statusText,
      verificationText,
      categoryText,
      descriptionText,
      authorText,
      dateText,
    ]
      .join(' ')
      .replace(/\s+/g, ' ')
      .toLowerCase();

    entries.push({
      id: requirementId,
      card,
      keyValue: keyText.toLowerCase(),
      keyNumeric: extractNumber(keyText),
      titleValue: titleText.toLowerCase(),
      statusValue: statusText.toLowerCase(),
      verificationValue: verificationText.toLowerCase(),
      authorValue: authorText.toLowerCase(),
      searchText,
      visible: true,
    });
  });

  state.cards = entries;
}

function collectTreeNodes(treeRoot) {
  if (!treeRoot) return;

  state.treeNodesMap.clear();
  const entries = [];
  
  treeRoot.querySelectorAll('[role="treeitem"]').forEach((node) => {
    const requirementId = node.dataset.requirementId;
    const statusId = node.dataset.status;
    const categoryId = node.dataset.category;
    const verificationId = node.dataset.verification;
    const searchText = (node.dataset.searchText || '').toLowerCase();

    const entry = {
      id: requirementId,
      node,
      statusId: statusId ? parseInt(statusId, 10) : null,
      categoryId: categoryId ? parseInt(categoryId, 10) : null,
      verificationId: verificationId ? parseInt(verificationId, 10) : null,
      searchText,
      visible: true,
    };
    
    entries.push(entry);
    state.treeNodesMap.set(node, entry);
  });
  
  state.treeNodes = entries;
}

function ensureNoResultsBanner() {
  if (state.noResultsBanner) {
    return state.noResultsBanner;
  }

  const host = document.querySelector('.reqman-requirements-table-section');
  if (!host) {
    return null;
  }

  const banner = document.createElement('div');
  banner.className = 'reqman-requirements-search-empty';
  banner.hidden = true;
  banner.innerHTML = `
    <strong>No matches.</strong>
    <div>Try a different keyword or clear your filters.</div>
  `;
  host.appendChild(banner);
  state.noResultsBanner = banner;
  return banner;
}

function updateNoResultsBanner(visible) {
  const banner = ensureNoResultsBanner();
  if (!banner) return;
  banner.hidden = !visible;
}

function applySearch(term = '') {
  state.searchTerm = term;
  const needle = normalize(term);
  let visibleCount = 0;

  // Apply to table rows
  state.rows.forEach((entry) => {
    const matches = !needle || entry.searchText.includes(needle);
    const wasVisible = entry.visible;
    entry.visible = matches;

    entry.row.classList.toggle('is-filtered-out', !matches);

    if (entry.detail) {
      const detailShouldHide = !matches || entry.detail.hasAttribute('hidden');
      entry.detail.classList.toggle('is-filtered-out', detailShouldHide);
    }

    if (matches) {
      visibleCount += 1;
      if (!wasVisible) {
        entry.row.classList.add('reqman-requirements-row--enter');
        requestAnimationFrame(() => entry.row.classList.remove('reqman-requirements-row--enter'));
      }
    }
  });

  // Apply to cards
  state.cards.forEach((entry) => {
    const matches = !needle || entry.searchText.includes(needle);
    const wasVisible = entry.visible;
    entry.visible = matches;

    entry.card.classList.toggle('is-filtered-out', !matches);

    if (matches) {
      visibleCount += 1;
      if (!wasVisible) {
        entry.card.style.animation = 'none';
        requestAnimationFrame(() => {
          entry.card.style.animation = '';
        });
      }
    }
  });

  // Apply to tree view
  if (state.treeRoot) {
    const matchCount = searchTree(state.treeRoot, needle);
    visibleCount += matchCount;
  }

  const totalEntries = state.rows.length + state.cards.length + state.treeNodes.length;
  updateNoResultsBanner(visibleCount === 0 && totalEntries > 0);
}

function applyFilters(filters = {}) {
  state.currentFilters = { ...state.currentFilters, ...filters };

  if (!state.treeRoot) return;

  const { status, verification, category } = state.currentFilters;
  
  filterTree(state.treeRoot, (node) => {
    if (status && node.dataset.status !== String(status)) return false;
    if (verification && node.dataset.verification !== String(verification)) return false;
    if (category && node.dataset.category !== String(category)) return false;
    return true;
  });
}

function debounce(fn, wait = 150) {
  let timeout = null;
  return (...args) => {
    window.clearTimeout(timeout);
    timeout = window.setTimeout(() => fn(...args), wait);
  };
}

function initSearch(input) {
  if (!input) {
    return;
  }

  const params = new URLSearchParams(window.location.search);
  const initial = params.get('search');
  if (initial) {
    input.value = initial;
  }

  const handler = debounce((event) => {
    applySearch(event.target.value);
  }, 120);
  input.addEventListener('input', handler);

  if (input.value) {
    applySearch(input.value);
  } else {
    applySearch('');
  }
}

function updateSortIndicators(table, activeKey, order) {
  table.querySelectorAll('th[data-sort-key]').forEach((th) => {
    const indicator = th.querySelector('.reqman-requirements-table__sort-indicator');
    if (th.dataset.sortKey === activeKey) {
      if (indicator) {
        indicator.textContent = order === 'asc' ? '↑' : '↓';
      }
      th.setAttribute('aria-sort', order === 'asc' ? 'ascending' : 'descending');
    } else {
      if (indicator) {
        indicator.textContent = '↕';
      }
      th.removeAttribute('aria-sort');
    }
  });
}

function sortRows(table, key) {
  if (!SORTERS[key]) {
    return;
  }

  const order =
    state.sortKey === key && state.sortOrder === 'asc' ? 'desc' : state.sortKey === key ? 'asc' : 'asc';

  const sorted = [...state.rows].sort((a, b) => {
    const result = SORTERS[key](a, b);
    return order === 'asc' ? result : -result;
  });

  const tbody = table.querySelector('tbody');
  sorted.forEach((entry) => {
    tbody.appendChild(entry.row);
    if (entry.detail) {
      tbody.appendChild(entry.detail);
    }
  });

  state.rows = sorted;
  state.sortKey = key;
  state.sortOrder = order;
  updateSortIndicators(table, key, order);
}

function initSorting(table) {
  if (!table) return;

  table.querySelectorAll('th[data-sort-key]').forEach((th) => {
    th.addEventListener('click', () => {
      sortRows(table, th.dataset.sortKey);
    });
  });
}

function toggleRowDetails(button) {
  const row = button.closest('.reqman-requirements-row');
  const targetId = button.getAttribute('aria-controls');
  const expanded = button.getAttribute('aria-expanded') === 'true';
  const nextState = !expanded;
  button.setAttribute('aria-expanded', String(nextState));

  if (!targetId) return;
  const detail = document.getElementById(targetId);
  if (!detail) return;

  detail.toggleAttribute('hidden', !nextState);
  if (nextState) {
    detail.classList.remove('is-filtered-out');
  }
}

function initRowDetails(table) {
  if (!table) return;

  table.addEventListener('click', (event) => {
    const trigger = event.target.closest('[data-action="toggle-row-details"]');
    if (!trigger || !table.contains(trigger)) {
      return;
    }
    event.preventDefault();
    toggleRowDetails(trigger);
  });
}

function initKeyboardShortcuts({ searchInput, newRequirementButton }) {
  document.addEventListener('keydown', (event) => {
    // Skip if focus is in input field
    const target = event.target;
    if (target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.tagName === 'SELECT' || target.isContentEditable)) {
      return;
    }

    if (event.key === '/' && searchInput) {
      event.preventDefault();
      searchInput.focus();
      searchInput.select();
    }

    if (event.key.toLowerCase() === 'n' && newRequirementButton) {
      event.preventDefault();
      newRequirementButton.click();
    }
  });
}

function syncCustomFiltersToHidden(form) {
  const hidden = form?.querySelector('#custom_filters_hidden');
  const inputs = form?.querySelectorAll('.custom-filter-input');
  if (!hidden || !inputs?.length) return;
  const arr = Array.from(inputs)
    .map((el) => ({
      field_id: parseInt(el.getAttribute('data-field-id'), 10),
      value: (el.value ?? '').trim(),
    }))
    .filter((item) => item.value !== '');
  hidden.value = arr.length ? JSON.stringify(arr) : '';
}

function initFiltersForm(form, searchInput) {
  if (!form) return;

  form.querySelectorAll('.custom-filter-input').forEach((el) => {
    el.addEventListener('change', () => syncCustomFiltersToHidden(form));
    el.addEventListener('input', () => syncCustomFiltersToHidden(form));
  });

  form.addEventListener('submit', () => syncCustomFiltersToHidden(form));

  form.querySelectorAll('[data-filter-control]').forEach((select) => {
    select.addEventListener('change', () => {
      // Update current filters
      const filterName = select.dataset.filterControl;
      const filterValue = select.value ? parseInt(select.value, 10) : null;
      
      if (filterName) {
        applyFilters({ [filterName]: filterValue });
      }
      
      renderFilterChips(form);
      
      // Removed auto-submit - user must click Apply button
    });
  });

  const clearButton = form.querySelector('[data-action="clear-filters"]');
  if (clearButton) {
    clearButton.addEventListener('click', (event) => {
      event.preventDefault();
      form.querySelectorAll('[data-filter-control]').forEach((select) => {
        select.value = '';
      });
      form.querySelectorAll('.custom-filter-input').forEach((el) => {
        el.value = '';
      });
      syncCustomFiltersToHidden(form);
      if (searchInput) {
        searchInput.value = '';
        applySearch('');
      }
      // Clear all filters
      applyFilters({ status: null, verification: null, category: null });
      renderFilterChips(form);
      
      // Use native form submission
      if (typeof form.requestSubmit === 'function') {
        form.requestSubmit();
      } else {
        form.submit();
      }
    });
  }

  renderFilterChips(form);
}

function renderFilterChips(form) {
  const container = document.getElementById('requirementsFilterChips');
  if (!form || !container) return;

  container.innerHTML = '';

  form.querySelectorAll('[data-filter-control]').forEach((control) => {
    const value = control.value;
    if (!value) return;

    const selectedOption = control.options[control.selectedIndex];
    if (!selectedOption) return;

    const prefix = control.dataset.filterLabel || control.name;
    const optionLabel = selectedOption.textContent.trim();
    if (!optionLabel) return;

    const chip = document.createElement('button');
    chip.type = 'button';
    chip.className = 'reqman-requirements-filter-chip';
    chip.dataset.filterKey = control.name;
    chip.dataset.action = 'remove-filter';
    chip.innerHTML = `${prefix}: ${optionLabel} <span aria-hidden="true">×</span><span class="u-visually-hidden">Clear ${prefix}: ${optionLabel}</span>`;
    container.appendChild(chip);
  });

  const hasChips = container.childElementCount > 0;
  container.hidden = !hasChips;

  if (hasChips) {
    container.querySelectorAll('[data-action="remove-filter"]').forEach((chip) => {
      chip.addEventListener('click', () => {
        const key = chip.dataset.filterKey;
        if (!key) return;
        const control = form.querySelector(`[name="${key}"]`);
        if (control) {
          control.value = '';
          renderFilterChips(form);
          
          // Use native form submission
          if (typeof form.requestSubmit === 'function') {
            form.requestSubmit();
          } else {
            form.submit();
          }
        }
      });
    });
  }
}

function buildDuplicateTitle(title) {
  if (!title) return 'Untitled requirement (Copy)';
  if (title.toLowerCase().includes('(copy')) {
    return `${title} (Copy)`;
  }
  return `${title} (Copy)`;
}

function buildDuplicateReference(reference) {
  if (!reference) return '';
  if (reference.toLowerCase().includes('copy')) {
    return `${reference}-${Date.now().toString().slice(-4)}`;
  }
  return `${reference}-COPY`;
}

async function duplicateRequirement(button) {
  const requirementId = button.dataset.requirementId;
  if (!requirementId) {
    return;
  }

  try {
    // Fetch the requirement data
    const requirement = await jsonFetch(`/api/requirements/${requirementId}`);
    
    // Show the modal with pre-filled data
    showDuplicateModal(requirement);
  } catch (error) {
    console.error('Failed to fetch requirement for duplication', error);
    const message = error?.message || 'Failed to load requirement data';
    showNotification(message, 'error');
  }
}

function showDuplicateModal(requirement) {
  const modal = document.getElementById('duplicateRequirementModal');
  if (!modal) {
    console.error('Duplicate modal not found');
    return;
  }

  // Pre-fill the form fields
  document.getElementById('dup_req_title').value = buildDuplicateTitle(requirement.title);
  document.getElementById('dup_req_reference').value = ''; // Leave blank for auto-generation
  document.getElementById('dup_req_description').value = requirement.description || '';
  document.getElementById('dup_req_justification').value = requirement.justification || '';
  document.getElementById('dup_req_category').value = requirement.category_id || '';
  document.getElementById('dup_req_current_status').value = requirement.status_id || '';
  const dupVerification = document.getElementById('dup_req_verification');
  if (dupVerification) {
    const ids = requirement.req_verification_ids || (requirement.verification_method_id ? [requirement.verification_method_id] : []);
    if (dupVerification.multiple) {
      Array.from(dupVerification.options).forEach((o) => {
        o.selected = ids.includes(parseInt(o.value, 10));
      });
    } else {
      dupVerification.value = ids.length ? String(ids[0]) : '';
    }
  }
  document.getElementById('dup_req_applicability').value = requirement.applicability_id || '';
  document.getElementById('dup_req_reviewer').value = requirement.reviewer_id || '';
  document.getElementById('dup_req_parent').value = requirement.parent_id || '0';
  document.getElementById('dup_project_id').value = requirement.project_id;
  document.getElementById('dup_req_author').value = requirement.author_id;
  
  // Load parent requirement options from the current page's requirements
  loadParentRequirementOptionsFromPage(requirement.id, requirement.parent_id);

  // Show the modal using Bootstrap
  if (typeof bootstrap !== 'undefined' && bootstrap.Modal) {
    const bsModal = new bootstrap.Modal(modal);
    bsModal.show();
  } else {
    modal.classList.add('show');
    modal.style.display = 'block';
    modal.setAttribute('aria-hidden', 'false');
    document.body.classList.add('modal-open');
  }
}

function loadParentRequirementOptionsFromPage(currentReqId, selectedValue) {
  const select = document.getElementById('dup_req_parent');
  
  // Clear existing options except the first one (None)
  while (select.options.length > 1) {
    select.remove(1);
  }
  
  // Get all requirements from the page
  const allRequirements = [];
  
  // Collect from table view rows
  state.rows.forEach(entry => {
    const row = entry.row;
    const reqId = parseInt(row.dataset.requirementId, 10);
    if (reqId && reqId !== currentReqId) {
      const titleCell = row.querySelector('.reqman-requirements-title');
      const keyCell = row.querySelector('.reqman-requirements-key__value');
      if (titleCell && keyCell) {
        allRequirements.push({
          id: reqId,
          reference: keyCell.textContent.trim(),
          title: titleCell.textContent.trim()
        });
      }
    }
  });
  
  // Collect from card view cards
  state.cards.forEach(entry => {
    const card = entry.card;
    const reqId = parseInt(card.dataset.requirementId, 10);
    if (reqId && reqId !== currentReqId) {
      const titleEl = card.querySelector('.reqman-requirement-card__title');
      const keyEl = card.querySelector('.reqman-requirement-card__key');
      if (titleEl && keyEl) {
        // Check if not already added
        if (!allRequirements.find(r => r.id === reqId)) {
          allRequirements.push({
            id: reqId,
            reference: keyEl.textContent.trim(),
            title: titleEl.textContent.trim()
          });
        }
      }
    }
  });
  
  // Collect from tree view nodes
  state.treeNodes.forEach(entry => {
    const node = entry.node;
    const reqId = parseInt(node.dataset.requirementId, 10);
    if (reqId && reqId !== currentReqId) {
      const titleEl = node.querySelector('.c-tree__title');
      const keyEl = node.querySelector('.c-tree__key');
      if (titleEl && keyEl) {
        // Check if not already added
        if (!allRequirements.find(r => r.id === reqId)) {
          allRequirements.push({
            id: reqId,
            reference: keyEl.textContent.trim(),
            title: titleEl.textContent.trim()
          });
        }
      }
    }
  });
  
  // Sort by ID
  allRequirements.sort((a, b) => a.id - b.id);
  
  // Add options to select
  allRequirements.forEach(req => {
    const option = document.createElement('option');
    option.value = req.id;
    option.textContent = `${req.reference} - ${req.title}`;
    if (req.id === selectedValue) {
      option.selected = true;
    }
    select.appendChild(option);
  });
}

/**
 * Updates data-requirement-preview-* attributes on the row's links so the hover card shows
 * current values after an inline edit.
 */
function updateRequirementPreviewInRow(row, field, displayText, projectId, parentId) {
  const titleLink = row.querySelector('a.reqman-requirements-title[data-requirement-preview]');
  if (field === 'status' && titleLink && displayText != null) {
    titleLink.setAttribute('data-requirement-preview-status', displayText);
  }
  if (field === 'category' && titleLink && displayText != null) {
    titleLink.setAttribute('data-requirement-preview-category', displayText);
  }
  if (field === 'parent') {
    const parentLink = row.querySelector('.reqman-requirements-row__cell--parent a[data-requirement-preview]');
    if (parentLink && projectId != null) {
      parentLink.setAttribute('data-requirement-preview-id', String(parentId ?? 0));
      parentLink.setAttribute('data-requirement-preview-project-id', String(projectId));
      if (parentId === 0 || !displayText) {
        parentLink.setAttribute('data-requirement-preview-title', '');
        parentLink.setAttribute('data-requirement-preview-ref', '');
        parentLink.removeAttribute('data-requirement-preview-description');
        parentLink.removeAttribute('data-requirement-preview-status');
        parentLink.removeAttribute('data-requirement-preview-category');
      } else {
        const parts = (displayText || '').split(/\s*—\s*/);
        const ref = parts.length > 1 ? parts[0].trim() : '';
        const title = parts.length > 1 ? parts.slice(1).join(' — ').trim() : displayText;
        parentLink.setAttribute('data-requirement-preview-title', title || '');
        parentLink.setAttribute('data-requirement-preview-ref', ref);
        parentLink.setAttribute('data-requirement-preview-description', '');
        parentLink.setAttribute('data-requirement-preview-status', '');
        parentLink.setAttribute('data-requirement-preview-category', '');
      }
    }
  }
}

function getInlineEditConfig() {
  const script = document.getElementById('requirementsInlineEditConfig');
  if (!script?.textContent) return { categories: [], statuses: [], verifications: [] };
  try {
    return JSON.parse(script.textContent.trim());
  } catch {
    return { categories: [], statuses: [], verifications: [] };
  }
}

function getParentOptionsForInlineEdit(currentReqId) {
  const options = [];
  const add = (list, getKey, getTitle, getRef) => {
    (list || []).forEach((entry) => {
      const row = entry.row || entry.card || entry.node;
      const reqId = parseInt(row?.dataset?.requirementId, 10);
      if (!reqId || reqId === currentReqId) return;
      const keyEl = row.querySelector?.('.reqman-requirements-key__value, .reqman-requirement-card__key, .c-tree__key');
      const titleEl = row.querySelector?.('.reqman-requirements-title, .reqman-requirement-card__title, .c-tree__title');
      const reference = keyEl?.textContent?.trim() || '';
      const title = titleEl?.textContent?.trim() || '';
      if (options.some((o) => o.id === reqId)) return;
      options.push({ id: reqId, reference, title });
    });
  };
  add(state.rows);
  add(state.cards);
  add(state.treeNodes);
  options.sort((a, b) => a.id - b.id);
  return options;
}

function openInlineEdit(cell, field, row, config) {
  const requirementId = parseInt(row.dataset.requirementId, 10);
  if (!requirementId) return;
  const displayEl = cell.querySelector('.reqman-requirements-cell__display');
  if (!displayEl || cell.querySelector('.reqman-inline-edit-select')) return;

  const select = document.createElement('select');
  select.className = 'reqman-inline-edit-select';
  select.setAttribute('aria-label', `Change ${field}`);

  if (field === 'category') {
    const currentId = parseInt(row.dataset.categoryId, 10) || 0;
    (config.categories || []).forEach((c) => {
      select.appendChild(new Option(c.title, String(c.id), false, c.id === currentId));
    });
    if (select.options.length === 0) select.appendChild(new Option('—', '0', false, true));
  } else if (field === 'status') {
    const currentId = parseInt(row.dataset.statusId, 10) || 0;
    (config.statuses || []).forEach((s) => {
      select.appendChild(new Option(s.title, String(s.id), false, s.id === currentId));
    });
  } else if (field === 'verification') {
    select.multiple = true;
    select.size = Math.min(5, (config.verifications || []).length + 1);
    const idsStr = (row.dataset.verificationIds || '').trim();
    const currentIds = idsStr ? idsStr.split(/\s+/).map((n) => parseInt(n, 10)).filter((n) => !Number.isNaN(n)) : [];
    (config.verifications || []).forEach((v) => {
      const opt = new Option(v.title, String(v.id), false, currentIds.includes(v.id));
      select.appendChild(opt);
    });
  } else if (field === 'parent') {
    const currentId = parseInt(row.dataset.parentId, 10) || 0;
    select.appendChild(new Option('None', '0', false, currentId === 0));
    getParentOptionsForInlineEdit(requirementId).forEach((req) => {
      const label = req.reference ? `${req.reference} — ${req.title}` : `RM-${req.id} — ${req.title}`;
      select.appendChild(new Option(label, String(req.id), false, req.id === currentId));
    });
  }

  const getCurrentValue = () => {
    if (field === 'verification') {
      return Array.from(select.selectedOptions).map((o) => o.value).sort().join(',');
    }
    return select.value;
  };
  const initialValue = getCurrentValue();

  let applied = false;
  const apply = async () => {
    if (applied) return;
    let payload;
    let displayText;
    let categoryId;
    let statusId;
    let verificationIds;
    let parentId;
    if (field === 'category') {
      const v = parseInt(select.value, 10) || 0;
      categoryId = v;
      payload = { category_id: v };
      const c = (config.categories || []).find((x) => x.id === v);
      displayText = c ? c.title : '—';
    } else if (field === 'status') {
      const v = parseInt(select.value, 10);
      if (Number.isNaN(v)) return;
      statusId = v;
      payload = { status_id: v };
      const s = (config.statuses || []).find((x) => x.id === v);
      displayText = s ? s.title : '—';
    } else if (field === 'verification') {
      const ids = Array.from(select.selectedOptions).map((o) => parseInt(o.value, 10)).filter((n) => n > 0);
      if (ids.length === 0) {
        showNotification('At least one verification method is required', 'error');
        return;
      }
      verificationIds = ids;
      payload = { verification_method_ids: ids };
      displayText = (config.verifications || []).filter((v) => ids.includes(v.id)).map((v) => v.title).join(', ') || '—';
    } else if (field === 'parent') {
      const v = parseInt(select.value, 10);
      if (Number.isNaN(v)) return;
      parentId = v;
      payload = { parent_id: v === 0 ? 0 : v };
      if (v === 0) {
        displayText = '—';
      } else {
        const opt = select.options[select.selectedIndex];
        displayText = opt ? opt.textContent.trim() : '—';
      }
    }
    applied = true;
    if (select.parentNode) select.remove();
    displayEl.hidden = false;
    try {
      await patchJson(`/api/requirements/${requirementId}`, payload);
      const projectId = window.__reqmanProjectId || '';
      if (field === 'category') {
        row.dataset.categoryId = String(categoryId ?? '');
        displayEl.textContent = displayText;
        updateRequirementPreviewInRow(row, 'category', displayText, projectId);
      } else if (field === 'status') {
        const statusDef = (config.statuses || []).find((x) => x.id === statusId);
        const tagColor = statusDef?.tag_color || null;
        row.dataset.statusId = String(statusId);
        row.dataset.statusLabel = displayText;
        displayEl.textContent = displayText;
        displayEl.dataset.status = displayText;
        displayEl.dataset.statusId = String(statusId);
        displayEl.className = 'reqman-requirements-status-badge reqman-requirements-cell__display reqman-requirements-status-badge--' + statusVariant(displayText);
        if (tagColor) {
          displayEl.style.backgroundColor = tagColor;
          displayEl.style.color = '#fff';
          displayEl.style.borderColor = tagColor;
        } else {
          displayEl.style.backgroundColor = '';
          displayEl.style.color = '';
          displayEl.style.borderColor = '';
        }
        updateRequirementPreviewInRow(row, 'status', displayText, projectId);
      } else if (field === 'verification') {
        row.dataset.verificationIds = (verificationIds || []).join(' ');
        displayEl.textContent = displayText;
      } else if (field === 'parent') {
        row.dataset.parentId = parentId === 0 ? '0' : String(parentId);
        displayEl.textContent = displayText;
        if (displayEl.tagName === 'A') {
          displayEl.href = parentId === 0 ? '#' : `/p/${projectId}/requirements/show/${parentId}`;
          displayEl.style.pointerEvents = parentId === 0 ? 'none' : 'auto';
        }
        updateRequirementPreviewInRow(row, 'parent', displayText, projectId, parentId);
      }
      showNotification('Updated successfully', 'success');
      if (field === 'status') decorateStatusBadges();
    } catch (err) {
      applied = false;
      const status = err?.response?.status;
      const msg = err?.message || 'Update failed';
      const detail = status ? ` (${status})` : '';
      showNotification(msg + detail, 'error');
      console.error('Requirement inline update failed:', err?.payload || err);
      window.location.reload();
    }
  };

  const onValueChange = () => {
    if (field !== 'verification') apply();
  };
  select.addEventListener('change', onValueChange);
  select.addEventListener('input', onValueChange);
  select.addEventListener('blur', () => {
    if (applied) return;
    if (field === 'verification') {
      apply();
      return;
    }
    if (getCurrentValue() !== initialValue) apply();
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

  if (field === 'verification') {
    select.addEventListener('dblclick', () => apply());
  }

  displayEl.hidden = true;
  cell.appendChild(select);
  select.focus();
}

function initInlineEdit(table) {
  if (!table) return;
  const config = getInlineEditConfig();
  const sc = document.getElementById('semanticSearchConfig');
  if (sc?.textContent) {
    try {
      const c = JSON.parse(sc.textContent.trim());
      window.__reqmanProjectId = c.projectId ?? '';
    } catch {
      window.__reqmanProjectId = '';
    }
  }
  table.addEventListener('click', (e) => {
    if (e.target.closest('.reqman-inline-edit-select')) return;
    const cell = e.target.closest('[data-inline-edit]');
    if (!cell || !table.contains(cell)) return;
    e.preventDefault();
    e.stopPropagation();
    const row = cell.closest('.reqman-requirements-row');
    if (!row) return;
    openInlineEdit(cell, cell.dataset.inlineEdit, row, config);
  });
}

function initDuplicateForm() {
  const form = document.getElementById('duplicateRequirementForm');
  if (!form) return;

  form.addEventListener('submit', async (event) => {
    event.preventDefault();
    
    const submitBtn = document.getElementById('duplicateSubmitBtn');
    const spinner = submitBtn.querySelector('.spinner-border');
    const originalText = submitBtn.textContent;
    
    // Disable button and show spinner
    submitBtn.disabled = true;
    if (spinner) spinner.style.display = 'inline-block';
    submitBtn.innerHTML = '<span class="spinner-border spinner-border-sm me-2" role="status" aria-hidden="true"></span>Creating...';
    
    try {
      const formData = new FormData(form);
      const payload = {
        id: null,
        title: formData.get('title'),
        description: formData.get('description'),
        reference_code: formData.get('reference_code') || '',
        justification: formData.get('justification') || '',
        category_id: parseInt(formData.get('category_id'), 10),
        status_id: parseInt(formData.get('status_id'), 10),
        verification_method_ids: formData.getAll('verification_method_ids').map((id) => parseInt(id, 10)).filter((n) => !Number.isNaN(n)),
        applicability_id: parseInt(formData.get('applicability_id'), 10),
        reviewer_id: parseInt(formData.get('reviewer_id'), 10) || 0,
        parent_id: parseInt(formData.get('parent_id'), 10) || 0,
        project_id: parseInt(formData.get('project_id'), 10),
        author_id: parseInt(formData.get('author_id'), 10),
      };

      await postJson('/api/requirements', payload);
      showNotification('Requirement duplicated successfully', 'success');
      
      // Close modal
      const modal = document.getElementById('duplicateRequirementModal');
      if (typeof bootstrap !== 'undefined' && bootstrap.Modal) {
        const bsModal = bootstrap.Modal.getInstance(modal);
        if (bsModal) bsModal.hide();
      } else {
        modal.classList.remove('show');
        modal.style.display = 'none';
        modal.setAttribute('aria-hidden', 'true');
        document.body.classList.remove('modal-open');
      }
      
      // Reload page after a short delay
      setTimeout(() => window.location.reload(), 600);
    } catch (error) {
      console.error('Failed to duplicate requirement', error);
      const message = error?.message || 'Failed to create duplicate requirement';
      showNotification(message, 'error');
      
      // Re-enable button
      submitBtn.disabled = false;
      submitBtn.textContent = originalText;
    }
  });
}

function initDuplicateButtons(container) {
  if (!container) return;

  container.addEventListener('click', (event) => {
    const trigger = event.target.closest('[data-action="duplicate-requirement"]');
    if (!trigger || !container.contains(trigger)) {
      return;
    }
    event.preventDefault();
    duplicateRequirement(trigger);
  });
}

function handleBadgeOverflow(card) {
  const metadata = card.querySelector('[data-badge-rail]');
  if (!metadata) return;

  const rail = metadata.querySelector('.reqman-requirement-card__badge-rail');
  const overflowChip = metadata.querySelector('[data-overflow]');
  if (!rail || !overflowChip) return;

  const badges = Array.from(rail.querySelectorAll('[data-badge]'));
  if (badges.length === 0) return;

  // Reset: show all badges
  badges.forEach((badge) => (badge.style.display = ''));
  overflowChip.hidden = true;

  // Check if overflow exists
  const railWidth = rail.offsetWidth;
  const availableWidth = metadata.offsetWidth;

  if (railWidth <= availableWidth) return;

  // Calculate how many badges fit, reserving space for +N chip (30px)
  const overflowChipWidth = 30;
  let visibleCount = 0;
  let accumulatedWidth = 0;

  for (let i = 0; i < badges.length; i++) {
    const badgeWidth = badges[i].offsetWidth + 4; // +4 for gap
    if (accumulatedWidth + badgeWidth + overflowChipWidth <= availableWidth) {
      accumulatedWidth += badgeWidth;
      visibleCount++;
    } else {
      break;
    }
  }

  // Hide overflow badges and show +N chip
  const hiddenCount = badges.length - visibleCount;
  if (hiddenCount > 0) {
    for (let i = visibleCount; i < badges.length; i++) {
      badges[i].style.display = 'none';
    }
    overflowChip.textContent = `+${hiddenCount}`;
    overflowChip.title = `${hiddenCount} more: ${badges
      .slice(visibleCount)
      .map((b) => b.textContent.trim())
      .join(', ')}`;
    overflowChip.hidden = false;
  }
}

function initBadgeOverflow() {
  const cards = document.querySelectorAll('.reqman-requirement-card');
  cards.forEach((card) => handleBadgeOverflow(card));

  // Re-calculate on window resize (debounced)
  let resizeTimeout;
  window.addEventListener('resize', () => {
    clearTimeout(resizeTimeout);
    resizeTimeout = setTimeout(() => {
      cards.forEach((card) => handleBadgeOverflow(card));
    }, 150);
  });
}

function initViewSwitcher() {
  const VIEW_KEY = 'requirements_view_preference';
  
  const cardBtn = document.getElementById('cardViewBtn');
  const tableBtn = document.getElementById('tableViewBtn');
  const treeBtn = document.getElementById('treeViewBtn');
  
  const cardView = document.getElementById('cardView');
  const tableView = document.getElementById('tableView');
  const treeView = document.getElementById('treeView');
  
  if (!cardBtn || !tableBtn || !treeBtn || !cardView || !tableView || !treeView) {
    return;
  }
  
  function switchView(viewName) {
    cardView.style.display = 'none';
    tableView.style.display = 'none';
    treeView.style.display = 'none';
    
    cardBtn.classList.remove('active');
    tableBtn.classList.remove('active');
    treeBtn.classList.remove('active');
    
    switch(viewName) {
      case 'card':
        cardView.style.display = 'block';
        cardBtn.classList.add('active');
        break;
      case 'tree':
        treeView.style.display = 'block';
        treeBtn.classList.add('active');
        if (window.__treeAPI) {
          setTimeout(() => window.__treeAPI.redrawConnectors(), 50);
        }
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
  treeBtn.addEventListener('click', () => switchView('tree'));
  
  try {
    const savedView = localStorage.getItem(VIEW_KEY) || 'table';
    switchView(savedView);
  } catch (e) {
    switchView('table');
  }
}

export function init() {
  const table = document.getElementById('requirementsTable');
  const cardsContainer = document.querySelector('.reqman-requirements-cards-grid');
  const treeRoot = document.querySelector('.c-tree');
  const searchInput = document.getElementById('requirementsSearch');
  const filtersForm = document.getElementById('requirementsFilterForm');
  const newRequirementButton = document.getElementById('newRequirementButton');

  state.statusMap = parseStatusDefinitions();
  state.treeRoot = treeRoot;
  
  // Initialize view switcher
  initViewSwitcher();
  
  // Initialize table view if present
  if (table) {
    collectRows(table);
    decorateStatusBadges();
    initSorting(table);
    initRowDetails(table);
    initDuplicateButtons(table);
    initInlineEdit(table);
    applySearch('');
  }

  // Initialize cards view if present
  if (cardsContainer) {
    collectCards(cardsContainer);
    decorateStatusBadges();
    initDuplicateButtons(cardsContainer);
    initBadgeOverflow();
    applySearch('');
  }

  // Initialize tree view if present
  if (treeRoot) {
    collectTreeNodes(treeRoot);
    decorateStatusBadges();
    applySearch('');
    
    // Initialize tree controls
    const treeAPI = initTreeControls({
      rootSelector: '.c-tree',
      toggleSelector: '[data-tree-toggle]',
      branchSelector: '[data-tree-branch]',
      expandAllSelector: '[data-tree-expand-all]',
      collapseAllSelector: '[data-tree-collapse-all]',
    });
    
    // Store tree API globally for connector redrawing
    if (treeAPI) {
      window.__treeAPI = treeAPI;
    }
    
    // Parse URL params for initial filters
    const params = new URLSearchParams(window.location.search);
    const statusFilter = params.get('status_filter');
    const verificationFilter = params.get('verification_filter');
    const categoryFilter = params.get('category_filter');
    
    if (statusFilter || verificationFilter || categoryFilter) {
      applyFilters({
        status: statusFilter ? parseInt(statusFilter, 10) : null,
        verification: verificationFilter ? parseInt(verificationFilter, 10) : null,
        category: categoryFilter ? parseInt(categoryFilter, 10) : null,
      });
    }
  }

  // Disable search if no data
  if (searchInput && !table && !cardsContainer && !treeRoot) {
    searchInput.disabled = true;
    searchInput.placeholder = 'No requirements to search yet';
  }

  initSearch(searchInput);
  initFiltersForm(filtersForm, searchInput);
  initKeyboardShortcuts({ searchInput, newRequirementButton });
  initDuplicateForm();
  
  // Initialize semantic search
  initSemanticSearch();
}
