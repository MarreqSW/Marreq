import { initRequirementReferenceValidation } from '../modules/referenceValidator.js';

function collectCategories(select) {
  return Array.from(select.options)
    .filter((option) => option.value)
    .map((option) => ({
      id: Number(option.value),
      tag: option.getAttribute('data-tag') || '',
    }));
}

export function init() {
  const form = document.querySelector('[data-requirement-form]');
  if (!form) {
    return;
  }

  const referenceInput = form.querySelector('#req_reference');
  const categorySelect = form.querySelector('#req_category');
  const errorEl = form.querySelector('#reference-error');
  const submitButton = form.querySelector('[data-role="submit-requirement"]');

  if (!referenceInput || !categorySelect || !errorEl || !submitButton) {
    return;
  }

  const categories = collectCategories(categorySelect);
  const allowSoftMismatch = form.getAttribute('data-allow-soft-mismatch') === 'true';

  initRequirementReferenceValidation({
    referenceSelector: referenceInput,
    categorySelector: categorySelect,
    errorSelector: errorEl,
    submitSelector: submitButton,
    categories,
    allowSoftMismatch,
  });
}

