const doc = document;

export const $ = (selector, root = doc) => root.querySelector(selector);
export const $$ = (selector, root = doc) => Array.from(root.querySelectorAll(selector));

export function on(root, event, selector, handler, options) {
  const target = root || doc;
  target.addEventListener(
    event,
    (nativeEvent) => {
      const match = nativeEvent.target.closest(selector);
      if (match && target.contains(match)) {
        handler(nativeEvent, match);
      }
    },
    options,
  );
}

export function dataSet(element, name, fallback = null) {
  if (!element) return fallback;
  const value = element.dataset ? element.dataset[name] : undefined;
  return value === undefined ? fallback : value;
}

export function toArray(value) {
  return Array.isArray(value) ? value : [value];
}

