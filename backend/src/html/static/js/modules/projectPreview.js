/**
 * Shows a project card preview when hovering over links that have project preview data.
 * Links must have: data-project-preview, data-project-preview-id, data-project-preview-name
 * Optional: data-project-preview-description
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

function getOrCreateTooltip() {
  if (tooltipEl) return tooltipEl;
  tooltipEl = document.createElement('div');
  tooltipEl.id = 'project-preview-card';
  tooltipEl.className = 'marreq-project-preview';
  tooltipEl.setAttribute('role', 'tooltip');
  tooltipEl.hidden = true;
  document.body.appendChild(tooltipEl);
  return tooltipEl;
}

function renderCard(link) {
  const id = link.getAttribute('data-project-preview-id');
  const name = link.getAttribute('data-project-preview-name') || 'Project';
  const description = link.getAttribute('data-project-preview-description') || '';
  const href = link.getAttribute('href') || `/p/${id}`;

  const descSnippet = description.length > 120 ? description.slice(0, 117) + '…' : description;
  const card = getOrCreateTooltip();
  card.innerHTML = `
    <div class="marreq-project-preview__bar" aria-hidden="true"></div>
    <div class="marreq-project-preview__content">
      <div class="marreq-project-preview__header">
        <span class="marreq-project-preview__icon" aria-hidden="true">${escapeHtml((name[0] || 'P').toUpperCase())}</span>
        <h4 class="marreq-project-preview__title">${escapeHtml(name)}</h4>
      </div>
      ${descSnippet ? `<p class="marreq-project-preview__description">${escapeHtml(descSnippet)}</p>` : ''}
      <a href="${escapeHtml(href)}" class="marreq-project-preview__action">View project</a>
    </div>
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
    link.setAttribute('aria-describedby', 'project-preview-card');
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
  const link = e.target.closest('[data-project-preview]');
  if (!link || !link.getAttribute('data-project-preview-id')) return;
  show(link);
}

function handleMouseLeave(e) {
  const link = e.target.closest('[data-project-preview]');
  const card = document.getElementById('project-preview-card');
  const related = e.relatedTarget;
  if (link && related && card && card.contains(related)) {
    return;
  }
  if (link) hide(link);
}

function handlePreviewCardMouseLeave(e) {
  const card = document.getElementById('project-preview-card');
  const related = e.relatedTarget;
  if (card && related && card.contains(related)) return;
  hide(null);
}

export function initProjectPreview() {
  document.body.addEventListener('mouseenter', handleMouseEnter, true);
  document.body.addEventListener('mouseleave', handleMouseLeave, true);

  document.body.addEventListener('mouseenter', (e) => {
    if (e.target.id === 'project-preview-card') {
      if (hideTimer) {
        clearTimeout(hideTimer);
        hideTimer = null;
      }
    }
  }, true);

  document.body.addEventListener('mouseleave', handlePreviewCardMouseLeave, true);
}
