/**
 * Shows a test card preview (same card view as the tests page) when hovering
 * over links that have test preview data.
 * Links must have: data-test-preview, data-test-preview-project-id, data-test-preview-id, data-test-preview-title
 * Optional: data-test-preview-ref, data-test-preview-description, data-test-preview-status, data-test-preview-category, data-test-preview-source
 */

const HOVER_DELAY_MS = 300;
const HIDE_DELAY_MS = 150;

let tooltipEl = null;
let showTimer = null;
let hideTimer = null;

function escapeHtml(text) {
  if (!text) return '';
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}

function testStatusToVariant(status) {
  if (!status) return 'default';
  const s = status.toLowerCase();
  if (s.includes('pass')) return 'accepted';
  if (s.includes('fail')) return 'rejected';
  if (s.includes('pending')) return 'proposal';
  if (s.includes('progress') || s.includes('in progress')) return 'draft';
  return 'default';
}

function getOrCreateTooltip() {
  if (tooltipEl) return tooltipEl;
  tooltipEl = document.createElement('div');
  tooltipEl.id = 'test-preview-card';
  tooltipEl.className = 'marreq-requirement-preview';
  tooltipEl.setAttribute('role', 'tooltip');
  tooltipEl.hidden = true;
  document.body.appendChild(tooltipEl);
  return tooltipEl;
}

const CATEGORY_ICON = '<svg class="marreq-requirement-card__badge-icon" width="10" height="10" fill="currentColor" viewBox="0 0 16 16"><path d="M0 2a2 2 0 0 1 2-2h12a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H2a2 2 0 0 1-2-2V2zm8.5 9.5a.5.5 0 0 1-1 0V5.707L5.354 7.854a.5.5 0 1 1-.708-.708l3-3a.5.5 0 0 1 .708 0l3 3a.5.5 0 0 1-.708.708L8.5 5.707V11.5z"/></svg>';
const VIEW_ICON = '<svg width="14" height="14" fill="currentColor" viewBox="0 0 16 16"><path d="M16 8s-3-5.5-8-5.5S0 8 0 8s3 5.5 8 5.5S16 8 16 8zM1.173 8a13.133 13.133 0 0 1 1.66-2.043C4.12 4.668 5.88 3.5 8 3.5c2.12 0 3.879 1.168 5.168 2.457A13.133 13.133 0 0 1 14.828 8c-.058.087-.122.183-.195.288-.335.48-.83 1.12-1.465 1.755C11.879 11.332 10.119 12.5 8 12.5c-2.12 0-3.879-1.168-5.168-2.457A13.134 13.134 0 0 1 1.172 8z"/><path d="M8 5.5a2.5 2.5 0 1 0 0 5 2.5 2.5 0 0 0 0-5zM4.5 8a3.5 3.5 0 1 1 7 0 3.5 3.5 0 0 1-7 0z"/></svg>';

function renderCard(link) {
  const projectId = link.getAttribute('data-test-preview-project-id');
  const id = link.getAttribute('data-test-preview-id');
  const ref = link.getAttribute('data-test-preview-ref') || '';
  const title = link.getAttribute('data-test-preview-title') || 'Test';
  const description = link.getAttribute('data-test-preview-description') || '';
  const status = link.getAttribute('data-test-preview-status') || '';
  const category = link.getAttribute('data-test-preview-category') || '';
  const source = link.getAttribute('data-test-preview-source') || '';
  const href = link.getAttribute('href') || (projectId && id ? `/p/${projectId}/verifications/show/${id}` : '#');

  const descSnippet = description.length > 120 ? description.slice(0, 117) + '…' : description;
  const displayRef = ref || `TEST-${id}`;
  const statusVariant = testStatusToVariant(status);
  const statusLabel = status || '—';
  const descriptionHtml = descSnippet
    ? `<p class="marreq-requirement-card__description">${escapeHtml(descSnippet)}</p>`
    : '<p class="marreq-requirement-card__description marreq-requirement-card__description--empty">—</p>';
  const categoryHtml = category
    ? `<span class="marreq-requirement-card__badge marreq-requirement-card__badge--category" data-badge title="Category: ${escapeHtml(category)}">${CATEGORY_ICON}${escapeHtml(category)}</span>`
    : '<span class="marreq-requirement-card__badge marreq-requirement-card__badge--muted" data-badge>—</span>';
  const sourceHtml = source
    ? `<span class="marreq-requirement-card__badge marreq-requirement-card__badge--verification" data-badge title="Source: ${escapeHtml(source)}">${escapeHtml(source)}</span>`
    : '';

  const card = getOrCreateTooltip();
  card.innerHTML = `
    <article class="marreq-requirement-card marreq-requirement-card--preview">
      <header class="marreq-requirement-card__header">
        <div class="marreq-requirement-card__reference">
          <span class="marreq-requirement-card__reference-text">${escapeHtml(displayRef)}</span>
        </div>
        <span class="marreq-requirements-status-badge marreq-requirements-status-badge--${escapeHtml(statusVariant)}" data-status="${escapeHtml(statusLabel)}">${escapeHtml(statusLabel)}</span>
      </header>
      <div class="marreq-requirement-card__body">
        <h3 class="marreq-requirement-card__title">
          <span class="marreq-requirement-card__title-link">${escapeHtml(title)}</span>
        </h3>
        ${descriptionHtml}
      </div>
      <div class="marreq-requirement-card__metadata" data-badge-rail>
        <div class="marreq-requirement-card__badge-rail">
          ${categoryHtml}
          ${sourceHtml}
        </div>
      </div>
      <footer class="marreq-requirement-card__footer">
        <div class="marreq-requirement-card__actions">
          <a href="${escapeHtml(href)}" class="marreq-requirement-card__action marreq-requirement-card__action--primary" title="View details">${VIEW_ICON}<span class="u-visually-hidden">View</span></a>
        </div>
      </footer>
    </article>
  `;
  return card;
}

function positionCard(card, anchor) {
  const rect = anchor.getBoundingClientRect();
  const cardRect = card.getBoundingClientRect();
  const viewport = { w: window.innerWidth, h: window.innerHeight };
  const margin = 8;

  let top = rect.bottom + margin;
  let left = rect.left;

  if (top + cardRect.height > viewport.h - margin) {
    top = rect.top - cardRect.height - margin;
  }
  if (top < margin) top = margin;

  if (left + cardRect.width > viewport.w - margin) {
    left = viewport.w - cardRect.width - margin;
  }
  if (left < margin) left = margin;

  card.style.top = `${top}px`;
  card.style.left = `${left}px`;
}

function show(link) {
  if (hideTimer) {
    clearTimeout(hideTimer);
    hideTimer = null;
  }
  showTimer = setTimeout(() => {
    showTimer = null;
    const card = renderCard(link);
    card.hidden = false;
    positionCard(card, link);
    link.setAttribute('aria-describedby', 'test-preview-card');
  }, HOVER_DELAY_MS);
}

function hide(link) {
  if (showTimer) {
    clearTimeout(showTimer);
    showTimer = null;
  }
  hideTimer = setTimeout(() => {
    hideTimer = null;
    const card = getOrCreateTooltip();
    card.hidden = true;
    if (link) link.removeAttribute('aria-describedby');
  }, HIDE_DELAY_MS);
}

function handleMouseEnter(e) {
  const link = e.target.closest('[data-test-preview]');
  if (!link || !link.getAttribute('data-test-preview-id')) return;
  show(link);
}

function handleMouseLeave(e) {
  const link = e.target.closest('[data-test-preview]');
  const card = document.getElementById('test-preview-card');
  const related = e.relatedTarget;
  if (link && related && card && card.contains(related)) {
    return;
  }
  if (link) hide(link);
}

function handlePreviewCardMouseLeave(e) {
  const card = document.getElementById('test-preview-card');
  const related = e.relatedTarget;
  if (card && related && card.contains(related)) return;
  hide(null);
}

export function initTestPreview() {
  document.body.addEventListener('mouseenter', handleMouseEnter, true);
  document.body.addEventListener('mouseleave', handleMouseLeave, true);

  document.body.addEventListener('mouseenter', (e) => {
    if (e.target.id === 'test-preview-card') {
      if (hideTimer) {
        clearTimeout(hideTimer);
        hideTimer = null;
      }
    }
  }, true);

  document.body.addEventListener('mouseleave', handlePreviewCardMouseLeave, true);
}
