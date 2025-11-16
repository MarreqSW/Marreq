/**
 * Enhanced Interactive Traceability Matrix
 * Features: Sticky headers, column filtering, fullscreen, keyboard shortcuts, tooltips
 */

import { initScrollIndicator } from '../modules/scrollIndicator.js';

// State management
const matrixState = {
  fullscreenEnabled: false,
  columnFiltersVisible: false,
  hiddenColumns: new Set(),
  currentSort: { by: '', order: 'asc' }
};

export function init() {
  const root = document.querySelector('#matrix-root');
  if (!root) {
    return;
  }

  // Initialize all matrix features
  initSortIndicators(root);
  initFilterHandlers();
  initColumnToggle();
  initFullscreen();
  initKeyboardShortcuts();
  initTableNavigation();
  initMatrixScrollIndicator();
  initTooltips();
  initCellInteractions();
  calculateCoverage();
  
  console.log('Matrix initialized with enhanced features');
}

/**
 * Calculate and display coverage percentage
 */
function calculateCoverage() {
  const coverageEl = document.getElementById('coveragePercentage');
  if (!coverageEl) return;
  
  const links = parseInt(coverageEl.dataset.links || '0');
  const reqs = parseInt(coverageEl.dataset.reqs || '0');
  const tests = parseInt(coverageEl.dataset.tests || '0');
  
  if (reqs > 0 && tests > 0) {
    const totalPossible = reqs * tests;
    const percentage = Math.round((links / totalPossible) * 100);
    coverageEl.textContent = `${percentage}%`;
    
    // Color code based on percentage
    if (percentage >= 80) {
      coverageEl.style.color = '#10b981'; // Green
    } else if (percentage >= 50) {
      coverageEl.style.color = '#f59e0b'; // Amber
    } else {
      coverageEl.style.color = '#ef4444'; // Red
    }
  } else {
    coverageEl.textContent = '0%';
  }
}

/**
 * Initialize sort indicators
 */
function initSortIndicators(root) {
  const currentSortBy = root.getAttribute('data-sort-by') || '';
  const currentSortOrder = root.getAttribute('data-sort-order') || 'asc';
  
  matrixState.currentSort = { by: currentSortBy, order: currentSortOrder };

  // Update sort indicators for test columns
  document.querySelectorAll('[data-test-indicator]').forEach((indicator) => {
    const testId = indicator.getAttribute('data-test-indicator');
    const testSortKey = `test_${testId}`;
    if (currentSortBy === testSortKey) {
      indicator.textContent = currentSortOrder === 'asc' ? '↑' : '↓';
    } else {
      indicator.textContent = '↕';
    }
  });
}

/**
 * Initialize filter form handlers
 */
function initFilterHandlers() {
  const form = document.getElementById('matrixFilterForm');
  if (!form) return;

  // Filters are now only applied when the "Apply" button is clicked
  // No auto-submit on filter changes

  // Search input - submit on Enter
  const searchInput = document.getElementById('search');
  if (searchInput) {
    searchInput.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        form.submit();
      }
    });

    // Optional: Live search feedback (visual only, not submitting)
    let debounceTimer;
    searchInput.addEventListener('input', (e) => {
      clearTimeout(debounceTimer);
      debounceTimer = setTimeout(() => {
        // Add visual feedback that search is ready
        if (e.target.value) {
          searchInput.style.borderColor = 'var(--primary-color)';
        } else {
          searchInput.style.borderColor = '';
        }
      }, 300);
    });
  }
}

/**
 * Initialize column visibility toggle
 */
function initColumnToggle() {
  const toggleButton = document.getElementById('toggleColumnFilters');
  const filtersPanel = document.getElementById('columnFiltersPanel');
  const showAllButton = document.getElementById('showAllColumns');
  const hideAllButton = document.getElementById('hideAllTests');
  
  if (toggleButton && filtersPanel) {
    toggleButton.addEventListener('click', () => {
      matrixState.columnFiltersVisible = !matrixState.columnFiltersVisible;
      filtersPanel.style.display = matrixState.columnFiltersVisible ? 'block' : 'none';
      toggleButton.classList.toggle('active', matrixState.columnFiltersVisible);
    });
  }

  // Show all columns
  if (showAllButton) {
    showAllButton.addEventListener('click', () => {
      document.querySelectorAll('[data-column-toggle]').forEach(checkbox => {
        checkbox.checked = true;
        const columnId = checkbox.dataset.columnToggle;
        matrixState.hiddenColumns.delete(columnId);
        showColumn(columnId);
      });
    });
  }

  // Hide all test columns
  if (hideAllButton) {
    hideAllButton.addEventListener('click', () => {
      document.querySelectorAll('[data-column-toggle]').forEach(checkbox => {
        checkbox.checked = false;
        const columnId = checkbox.dataset.columnToggle;
        matrixState.hiddenColumns.add(columnId);
        hideColumn(columnId);
      });
    });
  }

  // Individual column toggles
  document.querySelectorAll('[data-column-toggle]').forEach(checkbox => {
    checkbox.addEventListener('change', (e) => {
      const columnId = e.target.dataset.columnToggle;
      if (e.target.checked) {
        matrixState.hiddenColumns.delete(columnId);
        showColumn(columnId);
      } else {
        matrixState.hiddenColumns.add(columnId);
        hideColumn(columnId);
      }
    });
  });
}

function showColumn(columnId) {
  document.querySelectorAll(`[data-column-id="${columnId}"]`).forEach(cell => {
    cell.classList.remove('hidden');
    cell.style.display = '';
  });
}

function hideColumn(columnId) {
  document.querySelectorAll(`[data-column-id="${columnId}"]`).forEach(cell => {
    cell.classList.add('hidden');
    cell.style.display = 'none';
  });
}

/**
 * Initialize fullscreen mode
 */
function initFullscreen() {
  const toggleButton = document.getElementById('toggleFullscreen');
  const matrixCard = document.querySelector('.c-matrix-card');
  
  console.log('initFullscreen called');
  console.log('toggleButton:', toggleButton);
  console.log('matrixCard:', matrixCard);
  
  if (toggleButton && matrixCard) {
    console.log('Adding fullscreen event listener');
    toggleButton.addEventListener('click', (e) => {
      e.preventDefault();
      console.log('Fullscreen button clicked');
      matrixState.fullscreenEnabled = !matrixState.fullscreenEnabled;
      console.log('Fullscreen enabled:', matrixState.fullscreenEnabled);
      matrixCard.classList.toggle('is-fullscreen', matrixState.fullscreenEnabled);
      console.log('Matrix card classes:', matrixCard.className);
      
      // Update button icon or text
      const icon = toggleButton.querySelector('svg');
      if (matrixState.fullscreenEnabled) {
        toggleButton.title = 'Exit fullscreen';
        toggleButton.setAttribute('aria-label', 'Exit fullscreen');
      } else {
        toggleButton.title = 'Toggle fullscreen';
        toggleButton.setAttribute('aria-label', 'Toggle fullscreen');
      }
    });
  } else {
    console.error('Cannot initialize fullscreen: missing elements', {
      hasButton: !!toggleButton,
      hasCard: !!matrixCard
    });
  }
}

/**
 * Initialize keyboard shortcuts
 */
function initKeyboardShortcuts() {
  const searchInput = document.getElementById('search');
  const shortcutsPanel = document.getElementById('keyboardShortcutsPanel');
  const closeButton = document.getElementById('closeShortcuts');
  const overlay = document.getElementById('shortcutsOverlay');
  
  document.addEventListener('keydown', (e) => {
    // Ctrl/Cmd + K: Focus search
    if ((e.ctrlKey || e.metaKey) && e.key === 'k') {
      e.preventDefault();
      searchInput?.focus();
      return;
    }
    
    // Don't trigger shortcuts if typing in input
    if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA' || e.target.tagName === 'SELECT') {
      return;
    }
    
    // F: Toggle fullscreen
    if (e.key === 'f' || e.key === 'F') {
      e.preventDefault();
      document.getElementById('toggleFullscreen')?.click();
    }
    
    // ?: Show shortcuts help
    if (e.key === '?' && e.shiftKey) {
      e.preventDefault();
      if (shortcutsPanel) {
        shortcutsPanel.style.display = 'flex';
      }
    }
    
    // Escape: Close shortcuts
    if (e.key === 'Escape' && shortcutsPanel?.style.display === 'flex') {
      shortcutsPanel.style.display = 'none';
    }
  });
  
  // Close shortcuts panel
  if (closeButton && shortcutsPanel) {
    closeButton.addEventListener('click', () => {
      shortcutsPanel.style.display = 'none';
    });
  }
  
  if (overlay && shortcutsPanel) {
    overlay.addEventListener('click', () => {
      shortcutsPanel.style.display = 'none';
    });
  }
}

/**
 * Initialize keyboard navigation for table
 */
function initTableNavigation() {
  const table = document.getElementById('matrixTable');
  if (!table) return;
  
  table.addEventListener('keydown', (e) => {
    const target = e.target;
    
    // Allow Tab navigation through table
    if (e.key === 'Tab') {
      return;
    }
    
    // Arrow key navigation for cells
    if (['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight'].includes(e.key)) {
      const cell = target.closest('td, th');
      if (!cell) return;
      
      const row = cell.parentElement;
      const cellIndex = Array.from(row.children).indexOf(cell);
      const rowIndex = Array.from(row.parentElement.children).indexOf(row);
      
      let nextCell = null;
      
      if (e.key === 'ArrowRight') {
        nextCell = row.children[cellIndex + 1];
      } else if (e.key === 'ArrowLeft') {
        nextCell = row.children[cellIndex - 1];
      } else if (e.key === 'ArrowDown') {
        const nextRow = row.parentElement.children[rowIndex + 1];
        if (nextRow) nextCell = nextRow.children[cellIndex];
      } else if (e.key === 'ArrowUp') {
        const prevRow = row.parentElement.children[rowIndex - 1];
        if (prevRow) nextCell = prevRow.children[cellIndex];
      }
      
      if (nextCell) {
        e.preventDefault();
        const focusableElement = nextCell.querySelector('a') || nextCell;
        focusableElement.focus();
        
        // Scroll into view if needed
        nextCell.scrollIntoView({ block: 'nearest', inline: 'nearest', behavior: 'smooth' });
      }
    }
  });
  
  // Make cells focusable
  table.querySelectorAll('td, th').forEach(cell => {
    if (!cell.querySelector('a')) {
      cell.setAttribute('tabindex', '0');
    }
  });
}

/**
 * Initialize horizontal scroll indicator
 */
function initMatrixScrollIndicator() {
  initScrollIndicator({
    containerSelector: '.matrix-table-container',
    indicatorSelector: '#scrollIndicator',
    thumbSelector: '#scrollThumb',
  });
}

/**
 * Initialize tooltips for cells
 * Note: Tooltips are now handled via CSS using the data-tooltip attribute
 * This function is kept for potential future enhancements
 */
function initTooltips() {
  // Tooltips are now handled purely via CSS [data-tooltip] attribute
  // No JavaScript manipulation needed
}

/**
 * Initialize cell interactions
 */
function initCellInteractions() {
  const table = document.getElementById('matrixTable');
  if (!table) return;
  
  let selectedRow = null;
  
  // Add click handlers to all matrix cells (linked or not)
  document.querySelectorAll('.c-matrix-cell').forEach(cell => {
    cell.addEventListener('click', (e) => {
      // Don't trigger if clicking on a link inside the cell
      if (e.target.tagName === 'A') return;
      
      const row = cell.closest('tr');
      if (!row) return;
      
      // Remove selection from previously selected row
      if (selectedRow && selectedRow !== row) {
        selectedRow.classList.remove('is-selected');
      }
      
      // Toggle selection on current row
      if (selectedRow === row) {
        row.classList.remove('is-selected');
        selectedRow = null;
      } else {
        row.classList.add('is-selected');
        selectedRow = row;
      }
      
      // Log for linked cells
      if (cell.dataset.linked === 'true') {
        const reqId = cell.dataset.reqId;
        const testId = cell.dataset.testId;
        const testStatus = cell.dataset.testStatus;
        console.log(`Cell clicked: Req ${reqId} ↔ Test ${testId}, Status: ${testStatus}`);
      }
    });
  });
  
  // Highlight row and column on hover - optimized
  
  let currentHighlightedRow = null;
  let currentHighlightedCols = [];
  let currentFocusedCell = null;
  
  table.addEventListener('mouseover', (e) => {
    const cell = e.target.closest('td, th');
    if (!cell) return;
    
    const row = cell.parentElement;
    if (!row) return;
    
    const cellIndex = Array.from(row.children).indexOf(cell);
    
    // Skip if same cell to avoid unnecessary operations
    if (currentFocusedCell === cell) return;
    
    // Clear ALL previous highlights first
    if (currentHighlightedRow) {
      currentHighlightedRow.classList.remove('is-highlight');
    }
    currentHighlightedCols.forEach(c => c.classList.remove('is-highlight'));
    currentHighlightedCols = [];
    
    // Remove previous cell focus
    if (currentFocusedCell) {
      currentFocusedCell.classList.remove('is-focus');
    }
    
    // Add new highlights
    row.classList.add('is-highlight');
    currentHighlightedRow = row;
    
    // Highlight column - only for tbody and thead
    const tbody = table.querySelector('tbody');
    const thead = table.querySelector('thead');
    
    if (thead) {
      const headerRow = thead.querySelector('tr');
      if (headerRow && headerRow.children[cellIndex]) {
        const headerCell = headerRow.children[cellIndex];
        headerCell.classList.add('is-highlight');
        currentHighlightedCols.push(headerCell);
      }
    }
    
    if (tbody) {
      tbody.querySelectorAll('tr').forEach(r => {
        const targetCell = r.children[cellIndex];
        if (targetCell) {
          targetCell.classList.add('is-highlight');
          currentHighlightedCols.push(targetCell);
        }
      });
    }
    
    // Add focused highlight to current cell (will be on top due to CSS specificity)
    cell.classList.add('is-focus');
    currentFocusedCell = cell;
  });
  
  // Use mouseleave on the table for cleaner cleanup
  table.addEventListener('mouseleave', () => {
    // Clear all highlights when mouse leaves the table
    if (currentHighlightedRow) {
      currentHighlightedRow.classList.remove('is-highlight');
      currentHighlightedRow = null;
    }
    currentHighlightedCols.forEach(c => c.classList.remove('is-highlight'));
    currentHighlightedCols = [];
    
    if (currentFocusedCell) {
      currentFocusedCell.classList.remove('is-focus');
      currentFocusedCell = null;
    }
  });
}

// Export state for debugging
window.matrixState = matrixState;

