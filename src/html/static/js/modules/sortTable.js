function defaultAccessor(cell) {
  const input = cell.querySelector('input');
  if (input) return input.value;

  const select = cell.querySelector('select');
  if (select) {
    const option = select.options[select.selectedIndex];
    return option ? option.text : '';
  }

  const span = cell.querySelector('span');
  if (span) return span.textContent;

  return cell.textContent || '';
}

export function initTableSort(table, columnMap, options = {}) {
  if (!table) return;

  const accessor = options.accessor || defaultAccessor;
  const state = { column: null, order: 'asc' };

  table.addEventListener('click', (event) => {
    const trigger = event.target.closest('.c-table-sort-trigger');
    if (!trigger || !table.contains(trigger)) {
      return;
    }

    event.preventDefault();
    const columnKey = trigger.getAttribute('data-sort-key');
    if (!columnKey || !(columnKey in columnMap)) {
      return;
    }

    if (state.column === columnKey) {
      state.order = state.order === 'asc' ? 'desc' : 'asc';
    } else {
      state.column = columnKey;
      state.order = 'asc';
    }

    const rows = Array.from(table.querySelectorAll('tbody tr'));
    rows.sort((a, b) => {
      const aValue = accessor(a.cells[columnMap[columnKey]], a, columnKey).toLowerCase();
      const bValue = accessor(b.cells[columnMap[columnKey]], b, columnKey).toLowerCase();
      if (aValue < bValue) {
        return state.order === 'asc' ? -1 : 1;
      }
      if (aValue > bValue) {
        return state.order === 'asc' ? 1 : -1;
      }
      return 0;
    });

    const tbody = table.querySelector('tbody');
    rows.forEach((row) => tbody.appendChild(row));

    updateSortIndicators(table, columnKey, state.order);
  });
}

export function updateSortIndicators(table, activeKey, order) {
  table.querySelectorAll('.c-table-sort-trigger .c-table-sort-indicator').forEach((indicator) => {
    indicator.textContent = '↕';
  });

  const activeIndicator = table.querySelector(
    `.c-table-sort-trigger[data-sort-key="${activeKey}"] .c-table-sort-indicator`,
  );
  if (activeIndicator) {
    activeIndicator.textContent = order === 'asc' ? '↑' : '↓';
  }
}

