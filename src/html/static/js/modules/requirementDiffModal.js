/**
 * Renders and shows the requirement version diff modal (API response shape from diff API).
 * Used by requirement detail (Compare with current) and baseline detail (Diff vs current).
 */

function escapeHtml(text) {
  if (text == null) return '';
  return String(text)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}

function renderTextBlock(label, lines, textClass, bgClass) {
  if (!lines || lines.length === 0) return '';
  const content = lines.map((line) => escapeHtml(line)).join('\n');
  return `
    <div class="mb-2">
      <span class="small fw-semibold ${textClass}">${escapeHtml(label)}</span>
      <pre class="mb-0 p-2 rounded small ${bgClass}" style="white-space: pre-wrap; font-size: 0.875rem;">${content}</pre>
    </div>`;
}

function renderTextField(name, field) {
  if (!field || (!field.added?.length && !field.removed?.length && !field.unchanged?.length)) {
    return `<div class="mb-3"><h6 class="text-muted small text-uppercase">${escapeHtml(name)}</h6><p class="text-muted small mb-0">No changes.</p></div>`;
  }
  let html = `<div class="mb-3"><h6 class="text-muted small text-uppercase">${escapeHtml(name)}</h6>`;
  if (field.removed?.length) html += renderTextBlock('Removed', field.removed, 'text-danger', 'bg-danger bg-opacity-10');
  if (field.added?.length) html += renderTextBlock('Added', field.added, 'text-success', 'bg-success bg-opacity-10');
  if (field.unchanged?.length) html += renderTextBlock('Unchanged', field.unchanged, 'text-muted', 'bg-secondary bg-opacity-10');
  html += '</div>';
  return html;
}

function renderSingleValue(name, meta) {
  if (!meta) return '';
  if (meta.unchanged != null) {
    const label = meta.unchanged_label != null ? escapeHtml(meta.unchanged_label) : `id ${escapeHtml(String(meta.unchanged))}`;
    return `<div class="small"><span class="text-muted">${escapeHtml(name)}:</span> Unchanged (${label})</div>`;
  }
  const oldLabel = meta.old_label != null ? escapeHtml(meta.old_label) : (meta.old_id != null ? `id ${meta.old_id}` : '—');
  const newLabel = meta.new_label != null ? escapeHtml(meta.new_label) : (meta.new_id != null ? `id ${meta.new_id}` : '—');
  return `<div class="small"><span class="text-muted">${escapeHtml(name)}:</span> <span class="text-danger">${oldLabel}</span> → <span class="text-success">${newLabel}</span></div>`;
}

function renderVerification(ver) {
  if (!ver) return '';
  const parts = [];
  const added = (ver.added_labels?.length) ? ver.added_labels.map(escapeHtml).join(', ') : (ver.added_ids?.join(', ') ?? '');
  const removed = (ver.removed_labels?.length) ? ver.removed_labels.map(escapeHtml).join(', ') : (ver.removed_ids?.join(', ') ?? '');
  const unchanged = (ver.unchanged_labels?.length) ? ver.unchanged_labels.map(escapeHtml).join(', ') : (ver.unchanged_ids?.join(', ') ?? '');
  if (ver.added_ids?.length) parts.push(`<span class="text-success">Added: ${added}</span>`);
  if (ver.removed_ids?.length) parts.push(`<span class="text-danger">Removed: ${removed}</span>`);
  if (ver.unchanged_ids?.length) parts.push(`<span class="text-muted">Unchanged: ${unchanged}</span>`);
  if (parts.length === 0) return '<div class="small text-muted">No verification changes.</div>';
  return `<div class="small">${parts.join(' · ')}</div>`;
}

/**
 * Build HTML for the requirement diff modal body from the API payload.
 * @param {object} diff - API response: { text: { title, description }, metadata: { status, category, applicability, verification } }
 * @returns {string} HTML string
 */
export function renderRequirementDiff(diff) {
  if (!diff) return '<p class="text-muted">No diff data.</p>';
  let html = '';

  if (diff.text) {
    html += renderTextField('Title', diff.text.title);
    html += renderTextField('Description', diff.text.description);
  }

  if (diff.metadata) {
    html += '<div class="mb-3"><h6 class="text-muted small text-uppercase">Metadata</h6>';
    html += renderSingleValue('Status', diff.metadata.status);
    html += renderSingleValue('Category', diff.metadata.category);
    html += renderSingleValue('Applicability', diff.metadata.applicability);
    html += '<div class="mt-2">' + renderVerification(diff.metadata.verification) + '</div>';
    html += '</div>';
  }

  return html || '<p class="text-muted">No changes.</p>';
}

const MODAL_ID = 'requirementDiffModal';
const CONTENT_ID = 'requirementDiffContent';

/**
 * Shows the shared requirement diff modal with the given diff payload.
 * @param {object} diff - API response from version-diff or baseline-diff endpoint
 */
export function showRequirementDiff(diff) {
  const modalEl = document.getElementById(MODAL_ID);
  const contentEl = document.getElementById(CONTENT_ID);
  if (!modalEl || !contentEl || typeof window.bootstrap === 'undefined') return;
  contentEl.innerHTML = renderRequirementDiff(diff);
  const modal = window.bootstrap.Modal.getOrCreateInstance(modalEl);
  modal.show();
}
