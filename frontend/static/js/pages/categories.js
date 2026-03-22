import { initSearchFilter } from '../modules/tableFilter.js';

export function init() {
  const initialEmpty = document.getElementById('category-empty-state');
  const filteredEmpty = document.getElementById('category-empty-filter');

  if (initialEmpty) {
    initialEmpty.hidden = !initialEmpty.textContent.trim();
  }

  initSearchFilter({
    inputSelector: '#category-search-input',
    listSelector: '#category-list',
    itemSelector: '[data-category-row]',
    emptySelector: '#category-empty-filter',
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
