/**
 * Custom dropdown component – shared initializer.
 *
 * Uses a single delegated document listener for close-on-outside-click
 * instead of N listeners (one per dropdown instance).
 *
 * Depends on: css/50-components/dropdown.css  (c-custom-dropdown__*)
 */

/* ── Delegated close-on-outside-click (registered once) ────────── */
let delegatedListenerAttached = false;

function attachDelegatedListeners() {
  if (delegatedListenerAttached) return;
  delegatedListenerAttached = true;

  document.addEventListener('click', (event) => {
    document.querySelectorAll('[data-dropdown]').forEach((dropdown) => {
      const menu = dropdown.querySelector('[data-role="dropdown-menu"]');
      const trigger = dropdown.querySelector('[data-role="dropdown-trigger"]');
      if (menu && !menu.hidden && !dropdown.contains(event.target)) {
        menu.hidden = true;
        trigger?.setAttribute('aria-expanded', 'false');
      }
    });
  });

  document.addEventListener('keydown', (event) => {
    if (event.key !== 'Escape') return;
    document.querySelectorAll('[data-dropdown]').forEach((dropdown) => {
      const trigger = dropdown.querySelector('[data-role="dropdown-trigger"]');
      const menu = dropdown.querySelector('[data-role="dropdown-menu"]');
      if (trigger?.getAttribute('aria-expanded') === 'true') {
        menu.hidden = true;
        trigger.setAttribute('aria-expanded', 'false');
        trigger.focus();
      }
    });
  });
}

/* ── Per-dropdown setup ────────────────────────────────────────── */
function setupDropdown(dropdown) {
  const trigger = dropdown.querySelector('[data-role="dropdown-trigger"]');
  const menu = dropdown.querySelector('[data-role="dropdown-menu"]');
  const valueDisplay = dropdown.querySelector('[data-role="dropdown-value"]');
  const items = dropdown.querySelectorAll('.c-custom-dropdown__item');
  const hiddenSelect = dropdown.querySelector('select');
  const searchInput = dropdown.querySelector('[data-role="dropdown-search"]');

  if (!trigger || !menu || !valueDisplay || !hiddenSelect) return;

  const isMulti = hiddenSelect.multiple;

  // Tag items for multi-select CSS styling
  if (isMulti) {
    items.forEach((item) => item.setAttribute('data-multi-item', ''));
  }

  function closeMenu() {
    menu.hidden = true;
    trigger.setAttribute('aria-expanded', 'false');
    if (searchInput) {
      searchInput.value = '';
      filterItems('');
    }
  }

  function openMenu() {
    menu.hidden = false;
    trigger.setAttribute('aria-expanded', 'true');
    updateSelectedState();
    if (searchInput) {
      setTimeout(() => searchInput.focus(), 50);
    }
  }

  function filterItems(query) {
    const lowerQuery = query.toLowerCase();
    items.forEach((item) => {
      const searchText = (item.getAttribute('data-search-text') || item.textContent || '').toLowerCase();
      if (!query || searchText.includes(lowerQuery)) {
        item.classList.remove('c-custom-dropdown__item--hidden');
      } else {
        item.classList.add('c-custom-dropdown__item--hidden');
      }
    });
  }

  function updateSelectedState() {
    if (isMulti) {
      const selectedValues = new Set(
        Array.from(hiddenSelect.selectedOptions).map((o) => o.value),
      );
      items.forEach((item) => {
        if (selectedValues.has(item.getAttribute('data-value'))) {
          item.classList.add('c-custom-dropdown__item--selected');
        } else {
          item.classList.remove('c-custom-dropdown__item--selected');
        }
      });
    } else {
      const currentValue = hiddenSelect.value;
      items.forEach((item) => {
        if (item.getAttribute('data-value') === currentValue) {
          item.classList.add('c-custom-dropdown__item--selected');
        } else {
          item.classList.remove('c-custom-dropdown__item--selected');
        }
      });
    }
  }

  function updateDisplay() {
    if (isMulti) {
      const selected = Array.from(hiddenSelect.selectedOptions);
      if (selected.length === 0) {
        const placeholder = dropdown.dataset.placeholder || `Select ${dropdown.dataset.dropdown}...`;
        valueDisplay.textContent = placeholder;
        valueDisplay.classList.add('c-custom-dropdown__value--placeholder');
      } else if (selected.length === 1) {
        valueDisplay.textContent = selected[0].textContent.trim();
        valueDisplay.classList.remove('c-custom-dropdown__value--placeholder');
      } else {
        valueDisplay.textContent = `${selected.length} selected`;
        valueDisplay.classList.remove('c-custom-dropdown__value--placeholder');
      }
    } else {
      const selectedOption = hiddenSelect.options[hiddenSelect.selectedIndex];
      if (selectedOption && selectedOption.value) {
        valueDisplay.textContent = selectedOption.textContent.trim();
        valueDisplay.classList.remove('c-custom-dropdown__value--placeholder');
      } else {
        const placeholder = dropdown.dataset.dropdown;
        valueDisplay.textContent = `Select ${placeholder}...`;
        valueDisplay.classList.add('c-custom-dropdown__value--placeholder');
      }
    }
    updateSelectedState();
  }

  // Search functionality
  if (searchInput) {
    searchInput.addEventListener('input', (event) => {
      filterItems(event.target.value);
    });

    searchInput.addEventListener('keydown', (event) => {
      if (event.key === 'Escape') {
        closeMenu();
        trigger.focus();
      }
    });
  }

  // Toggle dropdown
  trigger.addEventListener('click', (event) => {
    event.preventDefault();
    const isOpen = trigger.getAttribute('aria-expanded') === 'true';

    // Close all other dropdowns first
    document.querySelectorAll('[data-dropdown]').forEach((otherDropdown) => {
      if (otherDropdown !== dropdown) {
        const otherTrigger = otherDropdown.querySelector('[data-role="dropdown-trigger"]');
        const otherMenu = otherDropdown.querySelector('[data-role="dropdown-menu"]');
        if (otherTrigger && otherMenu) {
          otherMenu.hidden = true;
          otherTrigger.setAttribute('aria-expanded', 'false');
        }
      }
    });

    if (isOpen) {
      closeMenu();
    } else {
      openMenu();
    }
  });

  // Handle item selection
  items.forEach((item) => {
    item.addEventListener('click', (event) => {
      event.preventDefault();
      const value = item.getAttribute('data-value');
      if (!value) return;

      if (isMulti) {
        // Toggle item in multi-select mode
        const option = hiddenSelect.querySelector(`option[value="${value}"]`);
        if (option) {
          option.selected = !option.selected;
        }
        hiddenSelect.dispatchEvent(new Event('change', { bubbles: true }));
        updateDisplay();
        // Don't close — let user pick more
      } else {
        hiddenSelect.value = value;

        // Copy data attributes from item to select option
        const tag = item.getAttribute('data-tag');
        const selectedOption = hiddenSelect.options[hiddenSelect.selectedIndex];
        if (tag && selectedOption) {
          selectedOption.setAttribute('data-tag', tag);
        }

        hiddenSelect.dispatchEvent(new Event('change', { bubbles: true }));
        updateDisplay();
        closeMenu();
      }
    });
  });

  // Initialize display from pre-selected value
  updateDisplay();
}

/* ── Public API ────────────────────────────────────────────────── */

/**
 * Initialise all custom dropdowns within a container.
 * @param {HTMLElement} root – typically the form element
 */
export function initCustomDropdowns(root) {
  attachDelegatedListeners();
  root.querySelectorAll('[data-dropdown]').forEach(setupDropdown);
}
