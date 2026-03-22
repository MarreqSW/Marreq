import { initSearchFilter } from '../modules/tableFilter.js';

export function init() {
  const initialEmpty = document.getElementById('group-empty-state');
  const filteredEmpty = document.getElementById('group-empty-filter');

  if (initialEmpty) {
    initialEmpty.hidden = !initialEmpty.textContent.trim();
  }

  initSearchFilter({
    inputSelector: '#group-search-input',
    listSelector: '#group-list',
    itemSelector: '[data-group-row]',
    emptySelector: '#group-empty-filter',
    onFilter: ({ count }) => {
      if (initialEmpty) {
        initialEmpty.hidden = true;
      }
      if (filteredEmpty) {
        filteredEmpty.hidden = count !== 0;
      }
    },
  });
}
