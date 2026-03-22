export function initSearchFilter({
  inputSelector,
  itemSelector,
  emptySelector,
  listSelector,
  getText = (element) => element.dataset.searchText || element.textContent || '',
  onFilter = () => {},
}) {
  const input = document.querySelector(inputSelector);
  const list = listSelector ? document.querySelector(listSelector) : document;
  if (!input || !list) {
    return;
  }

  const items = Array.from(list.querySelectorAll(itemSelector));
  if (!items.length) {
    return;
  }

  const emptyNotice = emptySelector ? document.querySelector(emptySelector) : null;

  function filter(value) {
    const query = value.trim().toLowerCase();
    let visibleCount = 0;

    items.forEach((item) => {
      const haystack = getText(item).toLowerCase();
      const match = haystack.includes(query);
      item.hidden = !match;
      if (match) {
        visibleCount += 1;
      }
    });

    if (emptyNotice) {
      emptyNotice.hidden = visibleCount !== 0;
    }

    onFilter({ count: visibleCount, query, items });
  }

  input.addEventListener('input', (event) => {
    filter(event.target.value);
  });

  return { filter };
}
