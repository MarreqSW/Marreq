/**
 * Enhanced Interactive Traceability Matrix
 * Features: Sticky headers, column filtering, fullscreen, keyboard shortcuts, tooltips
 */

import { initScrollIndicator } from '../modules/scrollIndicator.js';

// State management
const matrixState = {
  groupingEnabled: false,
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
  initGrouping();
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
  // Auto-submit on filter changes
  const filterSelect = document.getElementById('test_status_filter');
  if (filterSelect?.form) {
    filterSelect.addEventListener('change', () => {
      filterSelect.form.submit();
    });
  }

  const linkageFilter = document.getElementById('linkage_filter');
  if (linkageFilter?.form) {
    linkageFilter.addEventListener('change', () => {
      linkageFilter.form.submit();
    });
  }

  const perPageSelect = document.getElementById('per_page');
  if (perPageSelect?.form) {
    perPageSelect.addEventListener('change', () => {
      perPageSelect.form.submit();
    });
  }

  // Search input with debounce
  const searchInput = document.getElementById('search');
  if (searchInput) {
    let debounceTimer;
    searchInput.addEventListener('input', (e) => {
      clearTimeout(debounceTimer);
      debounceTimer = setTimeout(() => {
        // Could implement client-side filtering here for instant feedback
        console.log('Search:', e.target.value);
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
  const matrixCard = document.querySelector('.matrix-card');
  
  if (toggleButton && matrixCard) {
    toggleButton.addEventListener('click', () => {
      matrixState.fullscreenEnabled = !matrixState.fullscreenEnabled;
      matrixCard.classList.toggle('fullscreen', matrixState.fullscreenEnabled);
      
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
    
    // G: Toggle grouping
    if (e.key === 'g' || e.key === 'G') {
      e.preventDefault();
      document.getElementById('toggleGrouping')?.click();
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
 */
function initTooltips() {
  const cells = document.querySelectorAll('.matrix-cell[data-linked="true"]');
  cells.forEach(cell => {
    const testStatus = cell.dataset.testStatus;
    const reqId = cell.dataset.reqId;
    const testId = cell.dataset.testId;
    
    cell.addEventListener('mouseenter', (e) => {
      showTooltip(e.target, `Req ${reqId} ↔ Test ${testId}`);
    });
    
    cell.addEventListener('mouseleave', () => {
      hideTooltip();
    });
  });
}

let tooltipElement = null;

function showTooltip(target, text) {
  hideTooltip();
  
  tooltipElement = document.createElement('div');
  tooltipElement.className = 'matrix-tooltip';
  tooltipElement.textContent = text;
  tooltipElement.style.cssText = `
    position: absolute;
    background: rgba(0, 0, 0, 0.9);
    color: white;
    padding: 0.5rem 0.75rem;
    border-radius: 0.25rem;
    font-size: 0.875rem;
    pointer-events: none;
    z-index: 10000;
    white-space: nowrap;
  `;
  
  document.body.appendChild(tooltipElement);
  
  const rect = target.getBoundingClientRect();
  tooltipElement.style.top = `${rect.top - tooltipElement.offsetHeight - 8}px`;
  tooltipElement.style.left = `${rect.left + rect.width / 2 - tooltipElement.offsetWidth / 2}px`;
}

function hideTooltip() {
  if (tooltipElement) {
    tooltipElement.remove();
    tooltipElement = null;
  }
}

/**
 * Initialize grouping functionality
 */
function initGrouping() {
  const toggleButton = document.getElementById('toggleGrouping');
  
  if (toggleButton) {
    toggleButton.addEventListener('click', () => {
      matrixState.groupingEnabled = !matrixState.groupingEnabled;
      toggleButton.classList.toggle('active', matrixState.groupingEnabled);
      
      // Could implement actual grouping logic here
      // For now, just indicate the feature is toggled
      console.log('Grouping:', matrixState.groupingEnabled ? 'enabled' : 'disabled');
      
      // Could group by category, status, etc.
    });
  }
}

/**
 * Initialize cell interactions
 */
function initCellInteractions() {
  // Add click handlers to matrix cells
  document.querySelectorAll('.matrix-cell').forEach(cell => {
    if (cell.dataset.linked === 'true') {
      cell.style.cursor = 'pointer';
      cell.addEventListener('click', (e) => {
        const reqId = cell.dataset.reqId;
        const testId = cell.dataset.testId;
        
        // Could open a modal or navigate to detail view
        console.log(`Cell clicked: Req ${reqId} ↔ Test ${testId}`);
        
        // Add visual feedback
        cell.style.transform = 'scale(1.1)';
        setTimeout(() => {
          cell.style.transform = '';
        }, 200);
      });
    }
  });
  
  // Highlight row and column on hover
  const table = document.getElementById('matrixTable');
  if (table) {
    let currentHighlightedRow = null;
    let currentHighlightedCols = [];
    
    table.addEventListener('mouseover', (e) => {
      const cell = e.target.closest('td, th');
      if (!cell) return;
      
      const row = cell.parentElement;
      const cellIndex = Array.from(row.children).indexOf(cell);
      
      // Highlight row
      if (currentHighlightedRow && currentHighlightedRow !== row) {
        currentHighlightedRow.style.backgroundColor = '';
      }
      row.style.backgroundColor = 'var(--surface-hover)';
      currentHighlightedRow = row;
      
      // Highlight column
      currentHighlightedCols.forEach(c => c.style.backgroundColor = '');
      currentHighlightedCols = [];
      
      const allRows = table.querySelectorAll('tr');
      allRows.forEach(r => {
        const targetCell = r.children[cellIndex];
        if (targetCell) {
          targetCell.style.backgroundColor = 'var(--surface-hover)';
          currentHighlightedCols.push(targetCell);
        }
      });
    });
    
    table.addEventListener('mouseout', (e) => {
      if (!table.contains(e.relatedTarget)) {
        if (currentHighlightedRow) {
          currentHighlightedRow.style.backgroundColor = '';
          currentHighlightedRow = null;
        }
        currentHighlightedCols.forEach(c => c.style.backgroundColor = '');
        currentHighlightedCols = [];
      }
    });
  }
}

// Export state for debugging
window.matrixState = matrixState;

