/**
 * Pill-select component – toggle-able pill checkboxes.
 *
 * Each pill wraps a visually-hidden checkbox. Clicking the pill
 * toggles both the `is-selected` class and the checkbox state.
 *
 * Depends on: css/50-components/pill-select.css  (c-pill-select__*)
 */

/**
 * Initialise all pill-select widgets within a container.
 * @param {HTMLElement} root – typically the form element
 */
export function initPillSelects(root) {
  const widgets = root.querySelectorAll('.c-pill-select');

  widgets.forEach((widget) => {
    const pills = widget.querySelectorAll('.c-pill-select__pill');

    pills.forEach((pill) => {
      const checkbox = pill.querySelector('.c-pill-select__checkbox');
      if (!checkbox) return;

      pill.addEventListener('click', (event) => {
        // Don't toggle on link/button clicks that bubble up
        if (event.target.closest('a, button:not(.c-pill-select__pill)')) return;

        event.preventDefault();
        checkbox.checked = !checkbox.checked;
        pill.classList.toggle('is-selected', checkbox.checked);
      });

      // Keyboard: Enter/Space on the pill label
      pill.addEventListener('keydown', (event) => {
        if (event.key === ' ' || event.key === 'Enter') {
          event.preventDefault();
          checkbox.checked = !checkbox.checked;
          pill.classList.toggle('is-selected', checkbox.checked);
        }
      });
    });
  });
}

/**
 * Add a newly-created verification method pill to the first
 * pill-select widget found inside `root`.
 *
 * @param {HTMLElement} root – container (form)
 * @param {{ id: number|string, label: string }} data – new item
 * @param {boolean} [selected=true] – pre-select the new pill
 */
export function addPill(root, data, selected = true) {
  const list = root.querySelector('.c-pill-select__list');
  if (!list) return;

  const pill = document.createElement('label');
  pill.className = `c-pill-select__pill${selected ? ' is-selected' : ''}`;

  const checkbox = document.createElement('input');
  checkbox.type = 'checkbox';
  checkbox.name = 'verification_method_ids';
  checkbox.value = String(data.id);
  checkbox.checked = selected;
  checkbox.className = 'c-pill-select__checkbox';

  const span = document.createElement('span');
  span.className = 'c-pill-select__label';
  span.textContent = data.label;

  pill.appendChild(checkbox);
  pill.appendChild(span);

  // Wire up click toggling
  pill.addEventListener('click', (event) => {
    if (event.target.closest('a, button:not(.c-pill-select__pill)')) return;
    event.preventDefault();
    checkbox.checked = !checkbox.checked;
    pill.classList.toggle('is-selected', checkbox.checked);
  });

  list.appendChild(pill);
}
