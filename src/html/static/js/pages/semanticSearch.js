/**
 * Semantic Search Module
 * 
 * Provides AI-powered semantic search for requirements using:
 * - Hybrid search (lexical + vector similarity)
 * - Optional RAG answer generation with citations
 */

import { jsonFetch, postJson } from '../core/net.js';
import { showNotification } from '../modules/notifications.js';

// State
const state = {
  projectId: null,
  enabled: false,
  lastQuery: '',
  searchDebounceTimer: null,
};

// DOM Elements (cached after init)
let elements = {};

/**
 * Initialize the semantic search module.
 */
export function init() {
  // Get configuration from page
  const configEl = document.getElementById('semanticSearchConfig');
  if (configEl) {
    try {
      const config = JSON.parse(configEl.textContent);
      state.projectId = config.projectId;
    } catch (e) {
      console.error('Failed to parse semantic search config:', e);
    }
  }

  if (!state.projectId) {
    return; // No project context, disable semantic search
  }

  // Cache DOM elements
  elements = {
    modal: document.getElementById('semanticSearchModal'),
    openBtn: document.getElementById('semanticSearchBtn'),
    queryInput: document.getElementById('semanticSearchQuery'),
    submitBtn: document.getElementById('semanticSearchSubmit'),
    disabledAlert: document.getElementById('semanticSearchDisabled'),
    form: document.getElementById('semanticSearchForm'),
    loading: document.getElementById('semanticSearchLoading'),
    error: document.getElementById('semanticSearchError'),
    errorMessage: document.getElementById('semanticSearchErrorMessage'),
    answer: document.getElementById('semanticSearchAnswer'),
    answerText: document.getElementById('semanticSearchAnswerText'),
    citations: document.getElementById('semanticSearchCitations'),
    results: document.getElementById('semanticSearchResults'),
    resultsCount: document.getElementById('semanticSearchResultsCount'),
    resultsList: document.getElementById('semanticSearchResultsList'),
    empty: document.getElementById('semanticSearchEmpty'),
    // Filters
    statusFilter: document.getElementById('semanticStatusFilter'),
    categoryFilter: document.getElementById('semanticCategoryFilter'),
    applicabilityFilter: document.getElementById('semanticApplicabilityFilter'),
    verificationFilter: document.getElementById('semanticVerificationFilter'),
  };

  if (!elements.modal) {
    return; // Modal not present
  }

  // Check if semantic search is enabled
  checkSearchStatus();

  // Set up event listeners
  setupEventListeners();

  // Set up keyboard shortcuts
  setupKeyboardShortcuts();
}

/**
 * Check if semantic search is enabled on the server.
 */
async function checkSearchStatus() {
  try {
    const status = await jsonFetch(
      `/api/projects/${state.projectId}/requirements/semantic_search/status`
    );
    state.enabled = status.embeddings_enabled;

    if (!state.enabled) {
      showDisabledState();
    }
  } catch (e) {
    console.warn('Failed to check semantic search status:', e);
    state.enabled = false;
    showDisabledState();
  }
}

/**
 * Show disabled state message.
 */
function showDisabledState() {
  if (elements.disabledAlert) {
    elements.disabledAlert.classList.remove('d-none');
  }
  if (elements.form) {
    elements.form.classList.add('d-none');
  }
}

/**
 * Set up event listeners.
 */
function setupEventListeners() {
  // Submit button
  if (elements.submitBtn) {
    elements.submitBtn.addEventListener('click', performSearch);
  }

  // Enter key in query input
  if (elements.queryInput) {
    elements.queryInput.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        performSearch();
      }
    });

    // Debounced search on input (optional - uncomment to enable)
    // elements.queryInput.addEventListener('input', debounceSearch);
  }

  // Modal shown - focus input
  if (elements.modal) {
    elements.modal.addEventListener('shown.bs.modal', () => {
      if (elements.queryInput) {
        elements.queryInput.focus();
        elements.queryInput.select();
      }
    });

    // Modal hidden - reset state
    elements.modal.addEventListener('hidden.bs.modal', resetState);
  }
}

/**
 * Set up keyboard shortcuts.
 */
function setupKeyboardShortcuts() {
  document.addEventListener('keydown', (e) => {
    // Ctrl/Cmd + K to open modal
    if ((e.ctrlKey || e.metaKey) && e.key === 'k') {
      // Don't interfere with existing filter search when "/" is pressed
      const activeEl = document.activeElement;
      const isInputFocused = activeEl && (
        activeEl.tagName === 'INPUT' || 
        activeEl.tagName === 'TEXTAREA' || 
        activeEl.isContentEditable
      );

      if (!isInputFocused || e.key === 'k') {
        e.preventDefault();
        openModal();
      }
    }
  });
}

/**
 * Open the semantic search modal.
 */
function openModal() {
  if (!elements.modal || !state.projectId) return;

  // Use Bootstrap modal API if available
  if (typeof bootstrap !== 'undefined' && bootstrap.Modal) {
    const modal = bootstrap.Modal.getOrCreateInstance(elements.modal);
    modal.show();
  } else {
    // Fallback: trigger click on button
    elements.openBtn?.click();
  }
}

/**
 * Perform semantic search.
 */
async function performSearch() {
  const query = elements.queryInput?.value?.trim();
  if (!query) {
    showNotification('Please enter a search query', 'warning');
    return;
  }

  state.lastQuery = query;

  // Show loading state
  showLoadingState();

  try {
    // Build filter params
    const params = new URLSearchParams({
      q: query,
      k: '20',
    });

    if (elements.statusFilter?.value) {
      params.set('status_filter', elements.statusFilter.value);
    }
    if (elements.categoryFilter?.value) {
      params.set('category_filter', elements.categoryFilter.value);
    }
    if (elements.applicabilityFilter?.value) {
      params.set('applicability_filter', elements.applicabilityFilter.value);
    }
    if (elements.verificationFilter?.value) {
      params.set('verification_filter', elements.verificationFilter.value);
    }

    const response = await jsonFetch(
      `/api/projects/${state.projectId}/requirements/semantic_search?${params}`
    );

    if (!response.enabled) {
      showDisabledState();
      return;
    }

    // Render results
    renderResults(response.results, response.total);

    // Optionally get RAG answer for question-like queries
    if (isQuestionQuery(query)) {
      await getAnswer(query);
    }
  } catch (e) {
    showErrorState(e.message || 'Search failed');
  }
}

/**
 * Check if query looks like a question.
 */
function isQuestionQuery(query) {
  const questionWords = ['what', 'how', 'why', 'when', 'where', 'which', 'who', 'does', 'is', 'are', 'can', 'should'];
  const lowerQuery = query.toLowerCase();
  return questionWords.some(w => lowerQuery.startsWith(w + ' ')) || query.endsWith('?');
}

/**
 * Get RAG answer for a query.
 */
async function getAnswer(query) {
  try {
    const filters = {};
    if (elements.statusFilter?.value) {
      filters.status_filter = parseInt(elements.statusFilter.value, 10);
    }
    if (elements.categoryFilter?.value) {
      filters.category_filter = parseInt(elements.categoryFilter.value, 10);
    }
    if (elements.applicabilityFilter?.value) {
      filters.applicability_filter = parseInt(elements.applicabilityFilter.value, 10);
    }
    if (elements.verificationFilter?.value) {
      filters.verification_filter = parseInt(elements.verificationFilter.value, 10);
    }

    const response = await postJson(
      `/api/projects/${state.projectId}/requirements/ask`,
      { query, k: 10, ...filters }
    );

    renderAnswer(response.answer, response.citations);
  } catch (e) {
    // RAG might be disabled, just log and continue
    console.log('RAG answer not available:', e.message);
  }
}

/**
 * Show loading state.
 */
function showLoadingState() {
  hideAllStates();
  elements.loading?.classList.remove('d-none');
}

/**
 * Show error state.
 */
function showErrorState(message) {
  hideAllStates();
  if (elements.errorMessage) {
    elements.errorMessage.textContent = message;
  }
  elements.error?.classList.remove('d-none');
}

/**
 * Hide all state containers.
 */
function hideAllStates() {
  elements.loading?.classList.add('d-none');
  elements.error?.classList.add('d-none');
  elements.answer?.classList.add('d-none');
  elements.results?.classList.add('d-none');
  elements.empty?.classList.add('d-none');
}

/**
 * Render search results.
 */
function renderResults(results, total) {
  hideAllStates();

  if (!results || results.length === 0) {
    elements.empty?.classList.remove('d-none');
    return;
  }

  // Update count
  if (elements.resultsCount) {
    elements.resultsCount.textContent = total.toString();
  }

  // Clear previous results
  if (elements.resultsList) {
    elements.resultsList.innerHTML = '';

    // Render each result
    results.forEach((result, index) => {
      const item = createResultItem(result, index + 1);
      elements.resultsList.appendChild(item);
    });
  }

  elements.results?.classList.remove('d-none');
}

/**
 * Create a result item element.
 */
function createResultItem(result, rank) {
  const item = document.createElement('a');
  item.href = `/p/${state.projectId}/requirements/show/${result.id}`;
  item.className = 'list-group-item list-group-item-action';
  item.setAttribute('role', 'listitem');

  // Score badge color based on rank
  const scoreBadgeClass = rank <= 3 ? 'bg-success' : rank <= 10 ? 'bg-primary' : 'bg-secondary';

  item.innerHTML = `
    <div class="d-flex w-100 justify-content-between align-items-start">
      <div class="flex-grow-1 me-3">
        <div class="d-flex align-items-center mb-1">
          <span class="badge bg-light text-dark me-2 font-monospace">${escapeHtml(result.reference_code)}</span>
          <h6 class="mb-0">${escapeHtml(result.title)}</h6>
        </div>
        <p class="mb-1 text-muted small">${escapeHtml(result.snippet)}</p>
        <div class="d-flex flex-wrap gap-1 mt-2">
          <span class="badge bg-light text-dark">${escapeHtml(result.status)}</span>
          <span class="badge bg-light text-dark">${escapeHtml(result.category)}</span>
          <span class="badge bg-light text-dark">${escapeHtml(result.verification)}</span>
        </div>
      </div>
      <div class="text-end">
        <span class="badge ${scoreBadgeClass}" title="Match rank">#${rank}</span>
        ${result.lexical_rank ? `<div class="small text-muted mt-1">Lexical: #${result.lexical_rank}</div>` : ''}
        ${result.vector_rank ? `<div class="small text-muted">Vector: #${result.vector_rank}</div>` : ''}
      </div>
    </div>
  `;

  return item;
}

/**
 * Render RAG answer.
 */
function renderAnswer(answer, citations) {
  if (!answer || !elements.answer) return;

  if (elements.answerText) {
    elements.answerText.textContent = answer;
  }

  if (elements.citations && citations && citations.length > 0) {
    elements.citations.innerHTML = `
      <strong>Citations:</strong> 
      ${citations.map(c => 
        `<a href="/p/${state.projectId}/requirements/show/${c.requirement_id}" class="badge bg-light text-primary text-decoration-none me-1">[${escapeHtml(c.reference_code)}]</a>`
      ).join('')}
    `;
  } else if (elements.citations) {
    elements.citations.innerHTML = '';
  }

  elements.answer.classList.remove('d-none');
}

/**
 * Reset modal state.
 */
function resetState() {
  hideAllStates();
  if (elements.queryInput) {
    elements.queryInput.value = '';
  }
  if (elements.resultsList) {
    elements.resultsList.innerHTML = '';
  }
}

/**
 * Debounce search input.
 */
function debounceSearch() {
  if (state.searchDebounceTimer) {
    clearTimeout(state.searchDebounceTimer);
  }
  state.searchDebounceTimer = setTimeout(() => {
    const query = elements.queryInput?.value?.trim();
    if (query && query.length >= 3) {
      performSearch();
    }
  }, 300);
}

/**
 * Escape HTML to prevent XSS.
 */
function escapeHtml(str) {
  if (!str) return '';
  const div = document.createElement('div');
  div.textContent = str;
  return div.innerHTML;
}

// Export for testing
export const _internal = {
  state,
  performSearch,
  renderResults,
  isQuestionQuery,
  escapeHtml,
};
