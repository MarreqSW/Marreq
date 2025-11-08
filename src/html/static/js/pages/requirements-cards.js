/**
 * Requirements Card View - Modern card-based layout interactions
 * Extends the base requirements.js functionality for card layout
 */

import { jsonFetch, postJson } from '../core/net.js';
import { showNotification } from '../modules/notifications.js';

const state = {
  cards: [],
  searchTerm: '',
  statusFilter: null,
  verificationFilter: null,
  categoryFilter: null,
};

function normalize(text) {
  return (text || '').trim().toLowerCase();
}

/**
 * Initialize card view functionality
 */
function initCardView() {
  collectCards();
  attachFilterHandlers();
  attachSearchHandler();
  setupKeyboardShortcuts();
}

/**
 * Collect all requirement cards into state
 */
function collectCards() {
  const cardElements = document.querySelectorAll('.reqman-requirement-card');
  state.cards = Array.from(cardElements).map((card) => ({
    element: card,
    id: card.dataset.requirementId,
    statusId: card.dataset.statusId,
    title: normalize(card.querySelector('.reqman-requirement-card__title')?.textContent || ''),
    description: normalize(
      card.querySelector('.reqman-requirement-card__description')?.textContent || '',
    ),
    key: normalize(card.querySelector('.reqman-requirement-card__key')?.textContent || ''),
  }));
}

/**
 * Filter cards based on current state
 */
function filterCards() {
  let visibleCount = 0;

  state.cards.forEach(({ element, title, description, key, statusId }) => {
    const searchText = state.searchTerm.toLowerCase();
    const matchesSearch =
      !searchText ||
      title.includes(searchText) ||
      description.includes(searchText) ||
      key.includes(searchText);

    const matchesStatus = !state.statusFilter || statusId === state.statusFilter;

    if (matchesSearch && matchesStatus) {
      element.style.display = '';
      visibleCount++;
    } else {
      element.style.display = 'none';
    }
  });

  updateEmptyState(visibleCount);
}

/**
 * Update empty state message
 */
function updateEmptyState(visibleCount) {
  const grid = document.querySelector('.reqman-requirements-cards-grid');
  const paginationStatus = document.querySelector('.reqman-requirements-pagination__status');

  if (paginationStatus) {
    if (visibleCount === 0 && state.searchTerm) {
      paginationStatus.textContent = 'No requirements match your search';
    } else if (visibleCount === 1) {
      paginationStatus.textContent = 'Showing 1 requirement';
    } else {
      paginationStatus.textContent = `Showing ${visibleCount} requirements`;
    }
  }

  if (visibleCount === 0 && state.searchTerm && grid) {
    const existingEmpty = grid.querySelector('.reqman-requirements-search-empty');
    if (!existingEmpty) {
      const emptyDiv = document.createElement('div');
      emptyDiv.className = 'reqman-requirements-search-empty';
      emptyDiv.innerHTML = `
        <p>No requirements found matching "<strong>${escapeHtml(state.searchTerm)}</strong>"</p>
        <p class="text-muted">Try adjusting your search or filters</p>
      `;
      grid.appendChild(emptyDiv);
    }
  } else {
    const existingEmpty = grid?.querySelector('.reqman-requirements-search-empty');
    if (existingEmpty) {
      existingEmpty.remove();
    }
  }
}

/**
 * Escape HTML to prevent XSS
 */
function escapeHtml(text) {
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}

/**
 * Attach filter form handlers
 */
function attachFilterHandlers() {
  const form = document.getElementById('requirementsFilterForm');
  if (!form) return;

  const clearButton = form.querySelector('[data-action="clear-filters"]');
  if (clearButton) {
    clearButton.addEventListener('click', (e) => {
      e.preventDefault();
      form.reset();
      state.searchTerm = '';
      state.statusFilter = null;
      state.verificationFilter = null;
      state.categoryFilter = null;
      filterCards();
    });
  }

  // Handle select changes for client-side filtering
  const selects = form.querySelectorAll('select');
  selects.forEach((select) => {
    select.addEventListener('change', () => {
      const statusSelect = form.querySelector('#statusFilter');
      if (statusSelect) {
        state.statusFilter = statusSelect.value || null;
      }
      filterCards();
    });
  });
}

/**
 * Attach search input handler
 */
function attachSearchHandler() {
  const searchInput = document.getElementById('requirementsSearch');
  if (!searchInput) return;

  let debounceTimer;
  searchInput.addEventListener('input', (e) => {
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      state.searchTerm = e.target.value;
      filterCards();
    }, 200);
  });
}

/**
 * Setup keyboard shortcuts
 */
function setupKeyboardShortcuts() {
  const searchInput = document.getElementById('requirementsSearch');
  if (!searchInput) return;

  document.addEventListener('keydown', (e) => {
    // Focus search on '/' key
    if (e.key === '/' && document.activeElement !== searchInput) {
      e.preventDefault();
      searchInput.focus();
      searchInput.select();
    }

    // Clear search on Escape
    if (e.key === 'Escape' && document.activeElement === searchInput) {
      searchInput.value = '';
      state.searchTerm = '';
      searchInput.blur();
      filterCards();
    }
  });
}

/**
 * Initialize on DOM ready
 */
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', initCardView);
} else {
  initCardView();
}

export { initCardView, filterCards };
