function escapeHtml(text) {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}

function generateJsonDiff(oldObj, newObj) {
  const keys = new Set([...Object.keys(oldObj || {}), ...Object.keys(newObj || {})]);
  const oldLines = ['{'];
  const newLines = ['{'];

  keys.forEach((key) => {
    const oldValue = oldObj ? oldObj[key] : undefined;
    const newValue = newObj ? newObj[key] : undefined;

    if (JSON.stringify(oldValue) === JSON.stringify(newValue)) {
      const line = `  "${key}": ${JSON.stringify(oldValue)}`;
      oldLines.push(`<span class="text-muted">${escapeHtml(line)}</span>`);
      newLines.push(`<span class="text-muted">${escapeHtml(line)}</span>`);
    } else {
      if (oldValue !== undefined) {
        const line = `  "${key}": ${JSON.stringify(oldValue)}`;
        oldLines.push(`<span class="text-danger bg-danger bg-opacity-10">${escapeHtml(line)}</span>`);
      }
      if (newValue !== undefined) {
        const line = `  "${key}": ${JSON.stringify(newValue)}`;
        newLines.push(
          `<span class="text-success bg-success bg-opacity-10">${escapeHtml(line)}</span>`,
        );
      }
    }
  });

  oldLines.push('}');
  newLines.push('}');

  return { old: oldLines.join('\n'), new: newLines.join('\n') };
}

function generateLineDiff(oldText, newText) {
  const oldLines = (oldText || '').split('\n');
  const newLines = (newText || '').split('\n');

  const oldHighlighted = [];
  const newHighlighted = [];

  const length = Math.max(oldLines.length, newLines.length);
  for (let index = 0; index < length; index += 1) {
    const oldLine = oldLines[index];
    const newLine = newLines[index];

    if (oldLine === newLine) {
      const safe = escapeHtml(oldLine || '');
      oldHighlighted.push(`<span class="text-muted">${safe}</span>`);
      newHighlighted.push(`<span class="text-muted">${safe}</span>`);
    } else {
      if (oldLine !== undefined) {
        oldHighlighted.push(
          `<span class="text-danger bg-danger bg-opacity-10">${escapeHtml(oldLine)}</span>`,
        );
      }
      if (newLine !== undefined) {
        newHighlighted.push(
          `<span class="text-success bg-success bg-opacity-10">${escapeHtml(newLine)}</span>`,
        );
      }
    }
  }

  return { old: oldHighlighted.join('\n'), new: newHighlighted.join('\n') };
}

function buildDiff(oldValue, newValue) {
  try {
    const oldJson = oldValue ? JSON.parse(oldValue) : undefined;
    const newJson = newValue ? JSON.parse(newValue) : undefined;
    if (oldJson !== undefined || newJson !== undefined) {
      return generateJsonDiff(oldJson, newJson);
    }
  } catch (error) {
    /* fall back to line diff */
  }

  return generateLineDiff(oldValue || '', newValue || '');
}

export function initDiffModal({
  triggerSelector = '[data-action="show-changes"]',
  modalSelector = '#changesModal',
  contentSelector = '#changesContent',
}) {
  const modalElement = document.querySelector(modalSelector);
  const content = document.querySelector(contentSelector);

  if (!modalElement || !content || !window.bootstrap) {
    return;
  }

  const modal = new window.bootstrap.Modal(modalElement);

  document.querySelectorAll(triggerSelector).forEach((trigger) => {
    trigger.addEventListener('click', (event) => {
      event.preventDefault();
      const oldValues = trigger.getAttribute('data-old-values') || '';
      const newValues = trigger.getAttribute('data-new-values') || '';
      const diffHtml = buildDiff(oldValues, newValues);

      content.innerHTML = `
        <div class="row">
          <div class="col-6">
            <h6>Old Values</h6>
            <div class="bg-light p-2 rounded" style="max-height: 400px; overflow-y: auto; font-family: monospace; white-space: pre-wrap;">${diffHtml.old}</div>
          </div>
          <div class="col-6">
            <h6>New Values</h6>
            <div class="bg-light p-2 rounded" style="max-height: 400px; overflow-y: auto; font-family: monospace; white-space: pre-wrap;">${diffHtml.new}</div>
          </div>
        </div>
      `;

      modal.show();
    });
  });
}

