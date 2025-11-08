import { jsonFetch, postJson } from '../core/net.js';
import { showNotification } from '../modules/notifications.js';
import { searchTree, filterTree } from '../modules/tree.js';

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

function shouldIgnoreShortcut(target) {
  if (!target) return false;
  const tagName = target.tagName;
  return (
    tagName === 'INPUT' ||
    tagName === 'TEXTAREA' ||
    tagName === 'SELECT' ||
    target.isContentEditable
  );
}

function initKeyboardShortcuts({ searchInput, newRequirementButton }) {
  document.addEventListener('keydown', (event) => {
    if (event.key === '/' && !shouldIgnoreShortcut(event.target)) {
      event.preventDefault();
      if (searchInput) {
        searchInput.focus();
        searchInput.select();
      }
    }

    if (event.key.toLowerCase() === 'n' && !shouldIgnoreShortcut(event.target)) {
      if (newRequirementButton) {
        event.preventDefault();
        newRequirementButton.click();
      }
    }
  });
}

function requestSubmit(form) {
  if (!form) return;
  if (typeof form.requestSubmit === 'function') {
    form.requestSubmit();
  } else {
    form.submit();
  }
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
          requestSubmit(form);
        }
      });
    });
  }
}

function initFiltersForm(form, searchInput) {
  if (!form) return;

  form.querySelectorAll('[data-filter-control]').forEach((select) => {
    select.addEventListener('change', () => {
      // Update current filters
      const filterName = select.dataset.filterControl;
      const filterValue = select.value ? parseInt(select.value, 10) : null;
      
      if (filterName) {
        applyFilters({ [filterName]: filterValue });
      }
      
      renderFilterChips(form);
      requestSubmit(form);
    });
  });

  const clearButton = form.querySelector('[data-action="clear-filters"]');
  if (clearButton) {
    clearButton.addEventListener('click', (event) => {
      event.preventDefault();
      form.querySelectorAll('[data-filter-control]').forEach((select) => {
        select.value = '';
      });
      if (searchInput) {
        searchInput.value = '';
        applySearch('');
      }
      // Clear all filters
      applyFilters({ status: null, verification: null, category: null });
      renderFilterChips(form);
      requestSubmit(form);
    });
  }

  renderFilterChips(form);
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

  const originalLabel = button.textContent;
  button.disabled = true;
  button.textContent = 'Duplicating…';

  try {
    const requirement = await jsonFetch(`/api/requirements/${requirementId}`);
    const payload = {
      req_id: null,
      req_title: buildDuplicateTitle(requirement.req_title),
      req_description: requirement.req_description,
      req_verification: requirement.req_verification,
      req_author: requirement.req_author,
      req_link: requirement.req_link,
      req_category: requirement.req_category,
      req_current_status: requirement.req_current_status,
      req_parent: requirement.req_parent,
      req_reference: buildDuplicateReference(requirement.req_reference),
      req_reviewer: requirement.req_reviewer,
      req_applicability: requirement.req_applicability,
      req_justification: requirement.req_justification,
      project_id: requirement.project_id,
    };

    await postJson('/api/requirements', payload);
    showNotification('Requirement duplicated successfully', 'success');
    setTimeout(() => window.location.reload(), 600);
  } catch (error) {
    console.error('Failed to duplicate requirement', error);
    const message = error?.message || 'Failed to duplicate requirement';
    showNotification(message, 'error');
  } finally {
    button.disabled = false;
    button.textContent = originalLabel;
  }
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

function disableSearchWhenEmpty(searchInput, table) {
  if (!searchInput) return;
  if (!table) {
    searchInput.disabled = true;
    searchInput.placeholder = 'No requirements to search yet';
  }
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

export function init() {
  const table = document.getElementById('requirementsTable');
  const cardsContainer = document.querySelector('.reqman-requirements-cards-grid');
  const treeRoot = document.querySelector('.c-tree');
  const searchInput = document.getElementById('requirementsSearch');
  const filtersForm = document.getElementById('requirementsFilterForm');
  const newRequirementButton = document.getElementById('newRequirementButton');

  state.statusMap = parseStatusDefinitions();
  state.treeRoot = treeRoot;
  
  // Initialize table view if present
  if (table) {
    collectRows(table);
    decorateStatusBadges();
    initSorting(table);
    initRowDetails(table);
    initDuplicateButtons(table);
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

  disableSearchWhenEmpty(searchInput, table || cardsContainer || treeRoot);
  initSearch(searchInput);
  initFiltersForm(filtersForm, searchInput);
  initKeyboardShortcuts({ searchInput, newRequirementButton });
}
