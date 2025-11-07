function resolveElement(selectorOrElement) {
  if (!selectorOrElement) return null;
  if (selectorOrElement instanceof Element) return selectorOrElement;
  return document.querySelector(selectorOrElement);
}

export function setSelectValues(mapping) {
  Object.entries(mapping).forEach(([selector, value]) => {
    const element = resolveElement(selector);
    if (element) {
      element.value = String(value ?? '');
    }
  });
}

export function initRequirementReferenceValidation({
  referenceSelector,
  categorySelector,
  errorSelector,
  submitSelector,
  categories = [],
  collect,
  allowSoftMismatch = false,
}) {
  const referenceInput = resolveElement(referenceSelector);
  const categorySelect = resolveElement(categorySelector);
  const errorEl = resolveElement(errorSelector);
  const submitButton = resolveElement(submitSelector);

  if (!referenceInput || !categorySelect || !errorEl || !submitButton) {
    return;
  }

  function validate() {
    const availableCategories =
      typeof collect === 'function' ? collect() : categories;
    const reference = referenceInput.value.trim();
    const categoryId = Number.parseInt(categorySelect.value, 10);
    errorEl.hidden = true;
    errorEl.textContent = '';
    submitButton.disabled = false;

    if (!reference) {
      return true;
    }

    const category = availableCategories.find((entry) => entry.id === categoryId);
    if (!category) {
      return true;
    }

    const expectedPrefix = `REQ-${category.tag}-`;
    const pattern = new RegExp(`^REQ-${category.tag}-\\d+$`);

    if (!pattern.test(reference)) {
      errorEl.textContent = `Reference must follow the format "REQ-${category.tag}-NUMBER" (e.g., REQ-${category.tag}-1).`;
      errorEl.hidden = false;
      submitButton.disabled = !allowSoftMismatch;
      return false;
    }

    if (!reference.startsWith(expectedPrefix) && !allowSoftMismatch) {
      errorEl.textContent = `Reference must start with "${expectedPrefix}" for the selected category.`;
      errorEl.hidden = false;
      submitButton.disabled = true;
      return false;
    }

    if (!reference.startsWith(expectedPrefix) && allowSoftMismatch) {
      errorEl.textContent = `Warning: Reference "${reference}" doesn't match the selected category "${category.tag}". Consider updating the reference to match the category.`;
      errorEl.hidden = false;
    }

    return true;
  }

  referenceInput.addEventListener('input', validate);
  categorySelect.addEventListener('change', validate);
  validate();
}

export function configureRequirementDeleteButton({
  buttonSelector,
  statusId,
  isAdmin = false,
  allowedStatuses = [1, 2],
}) {
  const deleteButton = document.querySelector(buttonSelector);
  if (!deleteButton) {
    return;
  }

  const canDelete = isAdmin || allowedStatuses.includes(Number(statusId));
  deleteButton.style.display = canDelete ? 'inline-block' : 'none';
}
