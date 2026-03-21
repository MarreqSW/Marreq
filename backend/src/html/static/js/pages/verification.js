import { initSearchFilter } from '../modules/tableFilter.js';

export function init() {
  const initialEmpty = document.getElementById('verification-empty-state');
  const filteredEmpty = document.getElementById('verification-empty-filter');

  if (initialEmpty) {
    initialEmpty.hidden = !initialEmpty.textContent.trim();
  }

  initSearchFilter({
    inputSelector: '#verification-search-input',
    listSelector: '#verification-list',
    itemSelector: '[data-verification-row]',
    emptySelector: '#verification-empty-filter',
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
