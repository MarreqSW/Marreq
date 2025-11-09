import { initSearchFilter } from '../modules/tableFilter.js';

export function init() {
  const initialEmpty = document.getElementById('applicability-empty-state');
  const filteredEmpty = document.getElementById('applicability-empty-filter');

  if (initialEmpty) {
    initialEmpty.hidden = !initialEmpty.textContent.trim();
  }

  initSearchFilter({
    inputSelector: '#applicability-search-input',
    listSelector: '#applicability-list',
    itemSelector: '[data-applicability-row]',
    emptySelector: '#applicability-empty-filter',
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
