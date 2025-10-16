export function enableInlineTextEditing(container, selector, onCommit) {
  if (!container) return;

  container.querySelectorAll(selector).forEach((element) => {
    element.addEventListener('click', () => {
      if (element.querySelector('input')) {
        return;
      }

      const original = element.textContent.trim();
      const field = element.getAttribute('data-field');
      const id = element.getAttribute('data-id');
      if (!field || !id) {
        return;
      }

      const input = document.createElement('input');
      input.type = 'text';
      input.className = 'form-control form-control-sm c-form-control c-form-control--sm';
      input.value = original;

      element.textContent = '';
      element.appendChild(input);
      input.focus();
      input.select();

      const revert = () => {
        element.textContent = original;
      };

      const commit = () => {
        const nextValue = input.value.trim();
        element.textContent = nextValue;
        if (nextValue === original) {
          return;
        }
        onCommit({ id, field, value: nextValue, revert });
      };

      input.addEventListener('blur', commit);
      input.addEventListener('keydown', (event) => {
        if (event.key === 'Enter') {
          event.preventDefault();
          commit();
        } else if (event.key === 'Escape') {
          event.preventDefault();
          revert();
        }
      });
    });
  });
}

export function enableInlineChangeHandling(container, selector, onChange) {
  if (!container) return;

  container.querySelectorAll(selector).forEach((element) => {
    element.addEventListener('change', () => {
      const field = element.getAttribute('data-field');
      const id = element.getAttribute('data-id');
      if (!field || !id) {
        return;
      }

      onChange({ id, field, value: element.value });
    });
  });
}
