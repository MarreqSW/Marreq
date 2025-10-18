import { initRequirementReferenceValidation } from '../modules/referenceValidator.js';

function collectCategories(select) {
  return Array.from(select.options)
    .filter((option) => option.value)
    .map((option) => ({
      id: Number(option.value),
      tag: option.getAttribute('data-tag') || '',
    }));
}

function initReferenceValidation(form) {
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

function initStatusControls(form) {
  const toggle = form.querySelector('[data-role="status-toggle"]');
  const menu = form.querySelector('[data-role="status-menu"]');
  const statusLabel = toggle?.querySelector('.editor-status__label');
  const select = form.querySelector('#req_current_status');

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

  document.addEventListener('click', (event) => {
    if (!menu.hidden && !menu.contains(event.target) && event.target !== toggle) {
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

function initSaveMenu(form) {
  const trigger = form.querySelector('[data-role="save-menu-trigger"]');
  const panel = form.querySelector('[data-role="save-menu-panel"]');
  if (!trigger || !panel) {
    return;
  }

  function closeMenu() {
    panel.hidden = true;
    trigger.setAttribute('aria-expanded', 'false');
  }

  function openMenu() {
    panel.hidden = false;
    trigger.setAttribute('aria-expanded', 'true');
  }

  trigger.addEventListener('click', (event) => {
    event.preventDefault();
    const isOpen = trigger.getAttribute('aria-expanded') === 'true';
    if (isOpen) {
      closeMenu();
    } else {
      openMenu();
    }
  });

  document.addEventListener('click', (event) => {
    if (!panel.hidden && !panel.contains(event.target) && event.target !== trigger) {
      closeMenu();
    }
  });

  document.addEventListener('keydown', (event) => {
    if (event.key === 'Escape' && trigger.getAttribute('aria-expanded') === 'true') {
      closeMenu();
      trigger.focus();
    }
  });
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
    return '<p class="editor-preview__empty">Start by writing what the system <em>shall</em> do.</p>';
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
  const toolbarButtons = form.querySelectorAll('.editor-toolbar [data-format]');

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

function formatFileSize(bytes) {
  if (bytes < 1024) {
    return `${bytes} B`;
  }
  if (bytes < 1024 * 1024) {
    return `${(bytes / 1024).toFixed(1)} KB`;
  }
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function initAttachments(form) {
  const zone = form.querySelector('[data-role="attachments-zone"]');
  if (!zone) {
    return;
  }

  const input = zone.querySelector('[data-role="attachments-input"]');
  const browse = zone.querySelector('[data-role="browse-attachments"]');
  const list = zone.querySelector('[data-role="attachment-list"]');
  const state = [];

  function renderList() {
    if (!list) {
      return;
    }

    if (state.length === 0) {
      list.innerHTML = '';
      return;
    }

    list.innerHTML = state
      .map(
        (file) => `
          <li class="editor-dropzone__item">
            <span class="editor-dropzone__icon">${file.icon}</span>
            <div class="editor-dropzone__meta">
              <strong>${file.name}</strong>
              <small>${formatFileSize(file.size)} · ${file.status}</small>
            </div>
          </li>
        `,
      )
      .join('');
  }

  function updateFileStatus(id, status) {
    const file = state.find((item) => item.id === id);
    if (!file) {
      return;
    }
    file.status = status;
    renderList();
  }

  function simulateUpload(id) {
    updateFileStatus(id, 'uploading…');
    setTimeout(() => {
      updateFileStatus(id, 'ready for review');
    }, 600);
  }

  function handleFiles(files) {
    const nextFiles = Array.from(files);
    nextFiles.forEach((file) => {
      const id = `${file.name}-${file.lastModified}-${Math.random().toString(16).slice(2)}`;
      const icon = file.type.startsWith('image')
        ? '🖼'
        : file.type === 'application/pdf'
        ? '📄'
        : '📎';

      state.push({
        id,
        name: file.name,
        size: file.size,
        icon,
        status: 'queued',
      });

      renderList();
      simulateUpload(id);
    });
  }

  browse?.addEventListener('click', (event) => {
    event.preventDefault();
    input?.click();
  });

  input?.addEventListener('change', () => {
    if (!input.files) {
      return;
    }
    handleFiles(input.files);
    input.value = '';
  });

  zone.addEventListener('dragover', (event) => {
    event.preventDefault();
    zone.classList.add('editor-dropzone--active');
  });

  zone.addEventListener('dragleave', () => {
    zone.classList.remove('editor-dropzone--active');
  });

  zone.addEventListener('drop', (event) => {
    event.preventDefault();
    zone.classList.remove('editor-dropzone--active');
    if (event.dataTransfer?.files?.length) {
      handleFiles(event.dataTransfer.files);
    }
  });
}

function initLinkedRequirements(form) {
  const container = form.querySelector('[data-role="linked-requirements"]');
  if (!container) {
    return;
  }

  const searchInput = container.querySelector('[data-role="linked-search"]');
  const results = container.querySelector('[data-role="linked-results"]');
  const chips = container.querySelector('[data-role="linked-selected"]');
  const hiddenInput = container.querySelector('[data-role="linked-values"]');
  const parentSelect = form.querySelector('#req_parent');

  if (!searchInput || !results || !chips || !hiddenInput || !parentSelect) {
    return;
  }

  const optionData = Array.from(parentSelect.options)
    .filter((option) => option.value && option.value !== '0')
    .map((option) => ({
      id: Number(option.value),
      label: option.textContent.trim(),
      reference: option.dataset.reference || option.textContent.split('—')[0].trim(),
      title: option.dataset.title || option.textContent,
    }));

  const selected = new Map();
  syncHiddenInput();

  function syncHiddenInput() {
    const value = selected.size === 0 ? '' : Array.from(selected.keys()).join(',');
    hiddenInput.value = value;
    hiddenInput.dataset.snapshot = value;
    hiddenInput.setAttribute('disabled', '');
  }

  function renderChips() {
    if (selected.size === 0) {
      chips.innerHTML = '';
      return;
    }

    const template = Array.from(selected.entries())
      .map(
        ([id, label]) => `
          <li class="editor-linked__chip" data-id="${id}">
            <span>${label}</span>
            <button type="button" class="editor-linked__remove" aria-label="Remove ${label}">×</button>
          </li>
        `,
      )
      .join('');

    chips.innerHTML = template;
  }

  function addSelection(item) {
    if (selected.has(item.id)) {
      return;
    }
    selected.set(item.id, item.label);
    renderChips();
    syncHiddenInput();
  }

  function removeSelection(id) {
    selected.delete(id);
    renderChips();
    syncHiddenInput();
  }

  function renderResults(query) {
    const trimmed = query.trim().toLowerCase();
    if (!trimmed) {
      results.innerHTML = '';
      return;
    }

    const suggestions = optionData
      .filter(
        (option) =>
          option.label.toLowerCase().includes(trimmed) ||
          option.reference?.toLowerCase().includes(trimmed) ||
          option.title?.toLowerCase().includes(trimmed),
      )
      .slice(0, 5);

    if (suggestions.length === 0) {
      results.innerHTML =
        '<p class="editor-linked__empty">No matching requirements yet.</p>';
      return;
    }

    results.innerHTML = suggestions
      .map(
        (option) => `
          <button type="button" class="editor-linked__result" data-id="${option.id}">
            <span class="editor-linked__result-ref">${option.reference || `RM-${option.id}`}</span>
            <span class="editor-linked__result-title">${option.title}</span>
          </button>
        `,
      )
      .join('');
  }

  searchInput.addEventListener('input', () => {
    renderResults(searchInput.value);
  });

  searchInput.addEventListener('focus', () => {
    if (searchInput.value) {
      renderResults(searchInput.value);
    }
  });

  results.addEventListener('click', (event) => {
    const option = event.target.closest('[data-id]');
    if (!option) {
      return;
    }

    const id = Number(option.getAttribute('data-id'));
    const item = optionData.find((entry) => entry.id === id);
    if (item) {
      addSelection(item);
      searchInput.value = '';
      renderResults('');
      searchInput.focus();
    }
  });

  chips.addEventListener('click', (event) => {
    const removeButton = event.target.closest('.editor-linked__remove');
    if (!removeButton) {
      return;
    }

    const chip = removeButton.closest('[data-id]');
    if (!chip) {
      return;
    }

    const id = Number(chip.getAttribute('data-id'));
    removeSelection(id);
  });

  if (parentSelect.value && parentSelect.value !== '0') {
    const initial = optionData.find((item) => item.id === Number(parentSelect.value));
    if (initial) {
      addSelection(initial);
    }
  }
}

function formatTime(date) {
  return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
}

function initAutosave(form) {
  const autosaveText = form.querySelector('[data-role="autosave-text"]');
  const unsavedIndicator = form.querySelector('[data-unsaved-indicator]');
  const intervalField = form.querySelector('[data-role="autosave-interval"]');

  if (!autosaveText) {
    return;
  }

  const interval = Number(intervalField?.value || 30000);
  let autosaveTimer = null;
  let dirty = false;

  function markUnsaved() {
    dirty = true;
    if (autosaveText) {
      autosaveText.textContent = 'Unsaved changes';
    }
    unsavedIndicator?.removeAttribute('hidden');
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
      unsavedIndicator?.setAttribute('hidden', '');
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

export function init() {
  const form = document.querySelector('[data-requirement-form]');
  if (!form) {
    return;
  }

  initReferenceValidation(form);
  initStatusControls(form);
  initSaveMenu(form);
  initRichText(form);
  initRationale(form);
  initAttachments(form);
  initLinkedRequirements(form);
  initAutosave(form);
}
