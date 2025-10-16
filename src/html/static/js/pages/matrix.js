import { initScrollIndicator } from '../modules/scrollIndicator.js';

export function init() {
  const root = document.querySelector('#matrix-root');
  if (!root) {
    return;
  }

  const currentSortBy = root.getAttribute('data-sort-by') || '';
  const currentSortOrder = root.getAttribute('data-sort-order') || 'asc';
  const currentFilter = root.getAttribute('data-status-filter') || '';

  document.querySelectorAll('[data-test-indicator]').forEach((indicator) => {
    const testId = indicator.getAttribute('data-test-indicator');
    const testSortKey = `test_${testId}`;
    if (currentSortBy === testSortKey) {
      indicator.textContent = currentSortOrder === 'asc' ? '↑' : '↓';
    } else {
      indicator.textContent = '↕';
    }
  });

  document.querySelectorAll('[data-matrix-sort-link]').forEach((link) => {
    if (!currentFilter) {
      return;
    }
    const url = new URL(link.href, window.location.origin);
    url.searchParams.set('test_status_filter', currentFilter);
    link.href = url.toString();
  });

  const filterSelect = document.getElementById('test_status_filter');
  if (filterSelect && filterSelect.form) {
    filterSelect.addEventListener('change', () => {
      filterSelect.form.submit();
    });
  }

  initScrollIndicator({
    containerSelector: '.table-container',
    indicatorSelector: '#scrollIndicator',
    thumbSelector: '#scrollThumb',
  });
}

