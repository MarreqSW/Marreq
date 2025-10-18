import { jsonFetch, postJson } from '../core/net.js';
import { showNotification } from '../modules/notifications.js';

const state = {
  rows: [],
  sortKey: null,
  sortOrder: 'asc',
  searchTerm: '',
  statusMap: new Map(),
  noResultsBanner: null,
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

  updateNoResultsBanner(visibleCount === 0 && state.rows.length > 0);
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

function initFiltersForm(form, searchInput) {
  if (!form) return;

  form.querySelectorAll('[data-filter-control]').forEach((select) => {
    select.addEventListener('change', () => {
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
      requestSubmit(form);
    });
  }

  document.querySelectorAll('[data-action="remove-filter"]').forEach((chip) => {
    chip.addEventListener('click', () => {
      const key = chip.dataset.filterKey;
      if (!key) return;
      const control = form.querySelector(`[name="${key}"]`);
      if (control) {
        control.value = '';
        requestSubmit(form);
      }
    });
  });
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

function initDuplicateButtons(table) {
  if (!table) return;

  table.addEventListener('click', (event) => {
    const trigger = event.target.closest('[data-action="duplicate-requirement"]');
    if (!trigger || !table.contains(trigger)) {
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

export function init() {
  const table = document.getElementById('requirementsTable');
  const searchInput = document.getElementById('requirementsSearch');
  const filtersForm = document.getElementById('requirementsFilterForm');
  const newRequirementButton = document.getElementById('newRequirementButton');

  state.statusMap = parseStatusDefinitions();
  if (table) {
    collectRows(table);
    decorateStatusBadges();
    initSorting(table);
    initRowDetails(table);
    initDuplicateButtons(table);
    applySearch('');
  }

  disableSearchWhenEmpty(searchInput, table);
  initSearch(searchInput);
  initFiltersForm(filtersForm, searchInput);
  initKeyboardShortcuts({ searchInput, newRequirementButton });
}
