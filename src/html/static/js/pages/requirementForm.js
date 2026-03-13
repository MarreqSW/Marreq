import { initRequirementReferenceValidation } from '../modules/referenceValidator.js';
import { showNotification } from '../modules/notifications.js';
import { initCustomDropdowns } from '../modules/customDropdown.js';
import { initTraceBuilder, syncParentLinksFromList } from '../modules/traceBuilder.js';

function collectCategories(select) {
  return Array.from(select.options)
    .filter((option) => option.value)
    .map((option) => ({
      id: Number(option.value),
      tag: option.getAttribute('data-tag') || '',
    }));
}

function initReferenceValidation(form) {
  const referenceInput = form.querySelector('#reference_code');
  const categorySelect = form.querySelector('#category_id');
  const errorEl = form.querySelector('#reference-error');
  const submitButton = form.querySelector('[data-role="submit-requirement"]');

  if (!referenceInput || !categorySelect || !errorEl || !submitButton) {
    return;
  }

  const allowSoftMismatch = form.getAttribute('data-allow-soft-mismatch') === 'true';
  const collect = () => collectCategories(categorySelect);

  initRequirementReferenceValidation({
    referenceSelector: referenceInput,
    categorySelector: categorySelect,
    errorSelector: errorEl,
    submitSelector: submitButton,
    collect,
    allowSoftMismatch,
  });
}

function initStatusControls(form) {
  const toggle = form.querySelector('[data-role="status-toggle"]');
  const menu = form.querySelector('[data-role="status-menu"]');
  const statusLabel = toggle?.querySelector('.c-editor-status__label, .editor-status__label');
  const select = form.querySelector('#status_id');

  if (!toggle || !menu || !statusLabel || !select) {
    return;
  }

  function closeMenu() {
    menu.hidden = true;
    toggle.setAttribute('aria-expanded', 'false');
  }

  function openMenu() {
    menu.hidden = false;
    toggle.setAttribute('aria-expanded', 'true');
  }

  toggle.addEventListener('click', (event) => {
    event.preventDefault();
    const isOpen = toggle.getAttribute('aria-expanded') === 'true';
    if (isOpen) {
      closeMenu();
    } else {
      openMenu();
    }
  });

  menu.addEventListener('click', (event) => {
    const option = event.target.closest('[data-status-id]');
    if (!option) {
      return;
    }

    const value = option.getAttribute('data-status-id');
    if (value) {
      select.value = value;
      select.dispatchEvent(new Event('change', { bubbles: true }));
      statusLabel.textContent = option.textContent.trim();
    }

    closeMenu();
  });

  const statusControl = form.querySelector('[data-role="status-control"]');
  document.addEventListener('click', (event) => {
    if (!menu.hidden && statusControl && !statusControl.contains(event.target)) {
      closeMenu();
    }
  });

  document.addEventListener('keydown', (event) => {
    if (event.key === 'Escape' && toggle.getAttribute('aria-expanded') === 'true') {
      closeMenu();
      toggle.focus();
    }
  });

  select.addEventListener('change', () => {
    const selectedOption = select.options[select.selectedIndex];
    if (selectedOption) {
      statusLabel.textContent = selectedOption.textContent.trim();
    }
  });
}

function initInlineCreation(form) {
  const projectId = form.dataset.projectId;
  if (!projectId || !window.bootstrap) {
    return;
  }

  const config = {
    category: {
      label: 'Category',
      select: form.querySelector('#category_id'),
      modal: document.querySelector('#categoryModal'),
      form: document.querySelector('#inlineCategoryForm'),
      dropdown: form.querySelector('[data-dropdown="category"]'),
      endpoint: `/p/${projectId}/requirements/inline/category`,
      serialize: (fd) => ({
        title: (fd.get('title') || '').toString().trim(),
        description: (fd.get('description') || '').toString().trim(),
        tag: (fd.get('tag') || '').toString().trim(),
      }),
      apply: (data) => {
        const select = form.querySelector('#category_id');
        const dropdown = form.querySelector('[data-dropdown="category"]');
        
        if (!select || !dropdown) {
          return;
        }

        // Add to hidden select
        const option = document.createElement('option');
        option.value = String(data.id);
        if (data.tag) {
          option.dataset.tag = data.tag;
        }
        option.textContent = data.label;
        select.append(option);
        select.value = String(data.id);
        select.dispatchEvent(new Event('change', { bubbles: true }));
        
        // Add to dropdown list
        const list = dropdown.querySelector('[data-role="dropdown-list"]');
        if (list) {
          const button = document.createElement('button');
          button.type = 'button';
          button.className = 'c-custom-dropdown__item';
          button.setAttribute('data-value', String(data.id));
          if (data.tag) {
            button.setAttribute('data-tag', data.tag);
          }
          button.textContent = data.label;
          
          // Add click handler
          button.addEventListener('click', (event) => {
            event.preventDefault();
            select.value = String(data.id);
            select.dispatchEvent(new Event('change', { bubbles: true }));
            
            const valueDisplay = dropdown.querySelector('[data-role="dropdown-value"]');
            const menu = dropdown.querySelector('[data-role="dropdown-menu"]');
            const trigger = dropdown.querySelector('[data-role="dropdown-trigger"]');
            
            if (valueDisplay) {
              valueDisplay.textContent = data.label;
              valueDisplay.classList.remove('c-custom-dropdown__value--placeholder');
            }
            if (menu) {
              menu.hidden = true;
            }
            if (trigger) {
              trigger.setAttribute('aria-expanded', 'false');
            }
          });
          
          list.append(button);
        }
        
        // Update display
        const valueDisplay = dropdown.querySelector('[data-role="dropdown-value"]');
        if (valueDisplay) {
          valueDisplay.textContent = data.label;
          valueDisplay.classList.remove('c-custom-dropdown__value--placeholder');
        }

        const reference = form.querySelector('#reference_code');
        reference?.dispatchEvent(new Event('input', { bubbles: true }));
      },
    },
    applicability: {
      label: 'Applicability',
      select: form.querySelector('#applicability_id'),
      modal: document.querySelector('#applicabilityModal'),
      form: document.querySelector('#inlineApplicabilityForm'),
      dropdown: form.querySelector('[data-dropdown="applicability"]'),
      endpoint: `/p/${projectId}/requirements/inline/applicability`,
      serialize: (fd) => ({
        title: (fd.get('title') || '').toString().trim(),
        description: (fd.get('description') || '').toString().trim(),
        tag: (fd.get('tag') || '').toString().trim(),
      }),
      apply: (data) => {
        const select = form.querySelector('#applicability_id');
        const dropdown = form.querySelector('[data-dropdown="applicability"]');
        
        if (!select || !dropdown) {
          return;
        }

        // Add to hidden select
        const option = document.createElement('option');
        option.value = String(data.id);
        option.textContent = data.label;
        select.append(option);
        select.value = String(data.id);
        select.dispatchEvent(new Event('change', { bubbles: true }));
        
        // Add to dropdown list
        const list = dropdown.querySelector('[data-role="dropdown-list"]');
        if (list) {
          const button = document.createElement('button');
          button.type = 'button';
          button.className = 'c-custom-dropdown__item';
          button.setAttribute('data-value', String(data.id));
          button.textContent = data.label;
          
          // Add click handler
          button.addEventListener('click', (event) => {
            event.preventDefault();
            select.value = String(data.id);
            select.dispatchEvent(new Event('change', { bubbles: true }));
            
            const valueDisplay = dropdown.querySelector('[data-role="dropdown-value"]');
            const menu = dropdown.querySelector('[data-role="dropdown-menu"]');
            const trigger = dropdown.querySelector('[data-role="dropdown-trigger"]');
            
            if (valueDisplay) {
              valueDisplay.textContent = data.label;
              valueDisplay.classList.remove('c-custom-dropdown__value--placeholder');
            }
            if (menu) {
              menu.hidden = true;
            }
            if (trigger) {
              trigger.setAttribute('aria-expanded', 'false');
            }
          });
          
          list.append(button);
        }
        
        // Update display
        const valueDisplay = dropdown.querySelector('[data-role="dropdown-value"]');
        if (valueDisplay) {
          valueDisplay.textContent = data.label;
          valueDisplay.classList.remove('c-custom-dropdown__value--placeholder');
        }
      },
    },
    verification: {
      label: 'Verification methods',
      select: form.querySelector('#verification_method_ids'),
      modal: document.querySelector('#verificationModal'),
      form: document.querySelector('#inlineVerificationForm'),
      dropdown: form.querySelector('[data-dropdown="verification"]'),
      endpoint: `/p/${projectId}/requirements/inline/verification`,
      serialize: (fd) => ({
        title: (fd.get('title') || '').toString().trim(),
        description: (fd.get('description') || '').toString().trim(),
        tag: (fd.get('tag') || '').toString().trim(),
      }),
      apply: (data) => {
        const select = form.querySelector('#verification_method_ids');
        const dropdown = form.querySelector('[data-dropdown="verification"]');
        if (!select) return;

        // Add option to hidden select and mark selected
        const option = document.createElement('option');
        option.value = String(data.id);
        option.textContent = data.label;
        option.selected = true;
        select.append(option);
        select.dispatchEvent(new Event('change', { bubbles: true }));

        // Add visible item to dropdown list
        if (dropdown) {
          const list = dropdown.querySelector('[data-role="dropdown-list"]');
          if (list) {
            const button = document.createElement('button');
            button.type = 'button';
            button.className = 'c-custom-dropdown__item c-custom-dropdown__item--selected';
            button.setAttribute('data-value', String(data.id));
            button.setAttribute('data-multi-item', '');
            button.textContent = data.label;
            list.append(button);
          }
          // Reinitialise this dropdown so the new item gets a click handler
          initCustomDropdowns(dropdown);
          // Update trigger display
          const valueDisplay = dropdown.querySelector('[data-role="dropdown-value"]');
          const selectedCount = select.selectedOptions.length;
          if (valueDisplay && selectedCount > 0) {
            valueDisplay.textContent = selectedCount === 1
              ? select.selectedOptions[0].textContent.trim()
              : `${selectedCount} selected`;
            valueDisplay.classList.remove('c-custom-dropdown__value--placeholder');
          }
        }
      },
    },
  };

  Object.entries(config).forEach(([key, entry]) => {
    const { modal, form: modalForm, dropdown } = entry;
    
    if (!modal || !modalForm || !dropdown) {
      return;
    }
    
    const bootstrapModal = new window.bootstrap.Modal(modal);
    const actionButton = dropdown.querySelector(`[data-action="create-${key}"]`);
    
    if (actionButton) {
      actionButton.addEventListener('click', (event) => {
        event.preventDefault();
        event.stopPropagation();
        
        // Close dropdown menu
        const menu = dropdown.querySelector('[data-role="dropdown-menu"]');
        const trigger = dropdown.querySelector('[data-role="dropdown-trigger"]');
        if (menu) {
          menu.hidden = true;
        }
        if (trigger) {
          trigger.setAttribute('aria-expanded', 'false');
        }
        
        modalForm.reset();
        bootstrapModal.show();
      });
    }
    
    modalForm.addEventListener('submit', (event) => {
      event.preventDefault();
      const formData = new FormData(modalForm);
      const submitButton = modalForm.querySelector('button[type="submit"]');
      submitInline(entry, entry.serialize(formData), {
        submitButton,
        onSuccess: () => {
          bootstrapModal.hide();
          modalForm.reset();
        },
      });
    });
  });

  async function submitInline(entry, payload, options = {}) {
    const submitButton = options.submitButton;
    try {
      submitButton?.setAttribute('disabled', 'disabled');
      const response = await fetch(entry.endpoint, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(payload),
        credentials: 'include',
      });

      if (!response.ok) {
        throw new Error(`${entry.label} creation failed (${response.status})`);
      }

      const data = await response.json();
      entry.apply(data);
      if (typeof options.onSuccess === 'function') {
        options.onSuccess();
      }
      showNotification(`${entry.label} created`, 'success', { duration: 3500 });
    } catch (error) {
      showNotification(error.message || `Unable to create ${entry.label.toLowerCase()}`, 'error');
    } finally {
      submitButton?.removeAttribute('disabled');
    }
  }
}

function escapeHtml(value) {
  return value
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}

function renderPreviewContent(raw) {
  if (!raw) {
    return '<p class="c-editor-preview__empty">Start by writing what the system <em>shall</em> do.</p>';
  }

  const escaped = escapeHtml(raw);

  const withLinks = escaped.replace(/\[([^\]]+)\]\(([^)]+)\)/g, (_match, label, url) => {
    const safeLabel = label.trim();
    const safeUrl = url.trim();
    return `<a href="${safeUrl}" target="_blank" rel="noopener noreferrer">${safeLabel}</a>`;
  });

  const withBold = withLinks.replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>');
  const withItalic = withBold.replace(/(?:^|[^*])\*(?!\*)([^*]+)\*(?!\*)/g, (_match, text) => {
    return `<em>${text}</em>`;
  });
  const withCode = withItalic.replace(/`([^`]+)`/g, '<code>$1</code>');

  const lines = withCode.split('\n');
  const blocks = [];
  let buffer = [];

  lines.forEach((line) => {
    const trimmed = line.trim();
    if (trimmed.startsWith('- ') || trimmed.startsWith('* ')) {
      buffer.push(trimmed.substring(2).trim());
    } else {
      if (buffer.length > 0) {
        const listItems = buffer.map((item) => `<li>${item || '&nbsp;'}</li>`).join('');
        blocks.push(`<ul>${listItems}</ul>`);
        buffer = [];
      }
      if (trimmed.length > 0) {
        blocks.push(`<p>${trimmed}</p>`);
      }
    }
  });

  if (buffer.length > 0) {
    const listItems = buffer.map((item) => `<li>${item || '&nbsp;'}</li>`).join('');
    blocks.push(`<ul>${listItems}</ul>`);
  }

  return blocks.join('') || `<p>${withCode}</p>`;
}

function applyFormatting(textarea, format) {
  const { selectionStart, selectionEnd, value } = textarea;
  const selectedText = value.substring(selectionStart, selectionEnd) || '';

  function update(newText, startOffset = 0, endOffset = 0) {
    textarea.setRangeText(newText, selectionStart, selectionEnd, 'end');
    textarea.focus();
    const newCursor = selectionStart + startOffset;
    textarea.setSelectionRange(newCursor, newCursor + endOffset);
    textarea.dispatchEvent(new Event('input', { bubbles: true }));
  }

  switch (format) {
    case 'bold':
      update(`**${selectedText || 'text'}**`, 2, (selectedText || 'text').length);
      break;
    case 'italic':
      update(`*${selectedText || 'text'}*`, 1, (selectedText || 'text').length);
      break;
    case 'code':
      update(`\`${selectedText || 'snippet'}\``, 1, (selectedText || 'snippet').length);
      break;
    case 'list': {
      const lines = (selectedText || 'New item').split('\n');
      const formatted = lines.map((line) => {
        const trimmed = line.trim();
        if (trimmed.startsWith('- ') || trimmed.startsWith('* ')) {
          return line;
        }
        return trimmed.length > 0 ? `- ${trimmed}` : '- ';
      });
      update(formatted.join('\n'));
      break;
    }
    case 'link': {
      const label = selectedText || 'label';
      const url = window.prompt('Link URL');
      if (!url) {
        return;
      }
      const safeUrl = url.trim();
      if (!safeUrl) {
        return;
      }
      update(`[${label}](${safeUrl})`, 1, label.length);
      break;
    }
    default:
      break;
  }
}

function initRichText(form) {
  const textarea = form.querySelector('[data-role="requirement-input"]');
  const preview = form.querySelector('[data-role="preview"]');
  const previewContent = form.querySelector('[data-role="preview-content"]');
  const previewToggle = form.querySelector('[data-role="preview-toggle"]');
  const toolbarButtons = form.querySelectorAll('.c-editor-toolbar [data-format], .editor-toolbar [data-format]');

  if (!textarea || !preview || !previewContent || !previewToggle) {
    return;
  }

  toolbarButtons.forEach((button) => {
    button.addEventListener('click', (event) => {
      event.preventDefault();
      const format = button.getAttribute('data-format');
      if (format) {
        applyFormatting(textarea, format);
      }
    });
  });

  previewToggle.addEventListener('click', (event) => {
    event.preventDefault();
    const showingPreview = !preview.hidden;
    if (showingPreview) {
      preview.hidden = true;
      textarea.hidden = false;
      previewToggle.textContent = '👁 Preview';
      textarea.focus();
      return;
    }

    previewContent.innerHTML = renderPreviewContent(textarea.value);
    preview.hidden = false;
    textarea.hidden = true;
    previewToggle.textContent = '✍️ Edit';
  });

  textarea.addEventListener('input', () => {
    if (!preview.hidden) {
      previewContent.innerHTML = renderPreviewContent(textarea.value);
    }
  });
}

function initRationale(form) {
  const toggle = form.querySelector('[data-role="rationale-toggle"]');
  const panel = form.querySelector('[data-role="rationale-panel"]');

  if (!toggle || !panel) {
    return;
  }

  toggle.addEventListener('click', () => {
    const isHidden = panel.hasAttribute('hidden');
    if (isHidden) {
      panel.removeAttribute('hidden');
      toggle.setAttribute('aria-expanded', 'true');
    } else {
      panel.setAttribute('hidden', '');
      toggle.setAttribute('aria-expanded', 'false');
    }
  });
}

function initSuccessToast(form) {
  const message = form.dataset.flashSuccess;
  if (!message) {
    return;
  }

  showNotification(message, 'success', { duration: 3500 });
  delete form.dataset.flashSuccess;
}

function formatTime(date) {
  return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
}

function shouldPersistField(field) {
  if (!field || !field.name) {
    return false;
  }
  if (field.disabled) {
    return false;
  }
  if (field.type === 'hidden') {
    return false;
  }
  if (['author_id', 'project_id', 'req_author_email', 'id', 'intent'].includes(field.name)) {
    return false;
  }
  if (field.matches('[data-ignore-autosave]')) {
    return false;
  }
  return true;
}

function serializeFields(fields) {
  const snapshot = {};
  fields.forEach((field) => {
    if (field.type === 'checkbox' || field.type === 'radio') {
      snapshot[field.name] = field.checked;
    } else {
      snapshot[field.name] = field.value;
    }
  });
  return snapshot;
}

function applySnapshot(form, fields, snapshot) {
  fields.forEach((field) => {
    if (!(field.name in snapshot)) {
      return;
    }
    const value = snapshot[field.name];
    if (field.type === 'checkbox' || field.type === 'radio') {
      field.checked = Boolean(value);
      return;
    }
    const current = field.value?.trim?.() ?? field.value;
    if (field.tagName === 'SELECT') {
      field.value = String(value);
      field.dispatchEvent(new Event('change', { bubbles: true }));
      return;
    }
    if (!current) {
      field.value = value;
      field.dispatchEvent(new Event('input', { bubbles: true }));
    }
  });
}

function initCreateAutosave(form) {
  const projectId = form.dataset.projectId;
  const autosaveText = form.querySelector('[data-role="autosave-text"]');
  const indicator =
    form.querySelector('[data-role="autosave-indicator"]') ||
    form.querySelector('[data-unsaved-indicator]');

  if (!projectId || !autosaveText) {
    return;
  }

  const storageKey = `marreq:newRequirement:${projectId}`;
  const fields = Array.from(form.elements).filter(shouldPersistField);
  let restoring = false;
  let saveTimer = null;

  function writeSnapshot() {
    const payload = {
      savedAt: Date.now(),
      values: serializeFields(fields),
    };
    window.localStorage.setItem(storageKey, JSON.stringify(payload));
    indicator?.setAttribute('hidden', '');
    // autosaveText.textContent = `Draft saved locally · ${formatTime(new Date(payload.savedAt))}`;
  }

  function scheduleSave() {
    window.clearTimeout(saveTimer);
    saveTimer = window.setTimeout(writeSnapshot, 400);
  }

  function markDirty() {
    if (restoring) {
      return;
    }
    indicator?.removeAttribute('hidden');
    // autosaveText.textContent = 'Saving draft…';
    scheduleSave();
  }

  function restoreSnapshot() {
    const raw = window.localStorage.getItem(storageKey);
    if (!raw) {
      return;
    }

    try {
      const snapshot = JSON.parse(raw);
      if (snapshot?.values) {
        restoring = true;
        applySnapshot(form, fields, snapshot.values);
        restoring = false;
        showNotification('Restored unsaved draft for this project', 'info', {
          duration: 4000,
        });
      }
    } catch (error) {
      console.error('Failed to restore draft', error);
    }
  }

  restoreSnapshot();

  form.addEventListener('input', markDirty);
  form.addEventListener('change', markDirty);

  window.addEventListener('beforeunload', () => {
    if (indicator && indicator.hasAttribute('hidden')) {
      return;
    }
    window.clearTimeout(saveTimer);
    writeSnapshot();
  });

  form.addEventListener('submit', () => {
    window.clearTimeout(saveTimer);
    window.localStorage.removeItem(storageKey);
    indicator?.setAttribute('hidden', '');
  });

  const cancel = form.querySelector('[data-role="cancel-create"]');
  cancel?.addEventListener('click', () => {
    window.localStorage.removeItem(storageKey);
  });
}

function initEditorAutosave(form) {
  const autosaveText = form.querySelector('[data-role="autosave-text"]');
  const indicator =
    form.querySelector('[data-unsaved-indicator]') ||
    form.querySelector('[data-role="autosave-indicator"]');
  const intervalField = form.querySelector('[data-role="autosave-interval"]');

  if (!autosaveText) {
    return;
  }

  const interval = Number(intervalField?.value || 30000);
  let autosaveTimer = null;
  let dirty = false;

  function markUnsaved() {
    dirty = true;
    autosaveText.textContent = 'Unsaved changes';
    indicator?.removeAttribute('hidden');
    scheduleAutosave();
  }

  function scheduleAutosave() {
    if (!interval) {
      return;
    }
    window.clearTimeout(autosaveTimer);
    autosaveTimer = window.setTimeout(runAutosave, interval);
  }

  function runAutosave() {
    if (!dirty) {
      return;
    }
    autosaveText.textContent = 'Saving…';
    window.setTimeout(() => {
      dirty = false;
      indicator?.setAttribute('hidden', '');
      autosaveText.textContent = `Saved ✓ ${formatTime(new Date())}`;
    }, 600);
  }

  form.addEventListener('input', markUnsaved);
  form.addEventListener('change', markUnsaved);

  form.addEventListener('submit', () => {
    window.clearTimeout(autosaveTimer);
    autosaveText.textContent = 'Saving…';
  });
}

function initAutosave(form) {
  if (form.classList.contains('create-form')) {
    initCreateAutosave(form);
  } else {
    initEditorAutosave(form);
  }
}

function collectCustomFieldValues(form) {
  const hidden = form.querySelector('#custom_field_values');
  if (!hidden) return;
  const inputs = form.querySelectorAll('.c-reqform-field__custom-value');
  const arr = Array.from(inputs).map((el) => {
    const fieldId = Number(el.getAttribute('data-field-id'));
    const value = el.value?.trim() || null;
    return { field_id: fieldId, value };
  });
  hidden.value = JSON.stringify(arr);
}

export function init() {
  const form = document.querySelector('[data-requirement-form]');
  if (!form) {
    return;
  }

  const isCreateForm = form.classList.contains('create-form');

  form.addEventListener('submit', () => {
    collectCustomFieldValues(form);
    syncParentLinksFromList(form);
  });

  initCustomDropdowns(form);
  initReferenceValidation(form);
  if (isCreateForm) {
    initInlineCreation(form);
    initSuccessToast(form);
  }
  initTraceBuilder(form);
  initStatusControls(form);
  initRichText(form);
  initRationale(form);
  initAutosave(form);
}
