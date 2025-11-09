import { buildRequirementViewModel } from '../presenters/requirement.js';
import { initDiffModal } from '../modules/diffModal.js';

function parseCanonicalData() {
  const script = document.getElementById('requirement-detail-data');
  if (!script) {
    return null;
  }

  try {
    return JSON.parse(script.textContent || '{}');
  } catch (error) {
    console.error('Failed to parse requirement detail payload', error);
    return null;
  }
}

function getField(root, name) {
  return root.querySelector(`[data-field="${name}"]`);
}

function getFields(root, name) {
  return root.querySelectorAll(`[data-field="${name}"]`);
}

function getSlot(root, name) {
  return root.querySelector(`[data-slot="${name}"]`);
}

function toElementArray(element) {
  if (!element) {
    return [];
  }
  if (element instanceof Element) {
    return [element];
  }
  if (element instanceof NodeList || Array.isArray(element)) {
    return Array.from(element);
  }
  return [];
}

function setText(element, value) {
  const elements = toElementArray(element);
  elements.forEach((el) => {
    if (el) {
      el.textContent = value ?? '';
    }
  });
}

function renderBadge(element, badge) {
  if (!badge) {
    return;
  }
  toElementArray(element).forEach((el) => {
    if (!el) {
      return;
    }
    el.className = `badge ${badge.variant}`;
    el.textContent = badge.label;
  });
}

function renderSolidity(root, solidity) {
  if (!solidity) {
    return;
  }
  const labelEl = getField(root, 'solidity-label');
  const descriptionEl = getField(root, 'solidity-description');

  if (labelEl) {
    labelEl.className = `${solidity.variant} fw-semibold`;
    labelEl.textContent = solidity.label;
  }

  if (descriptionEl) {
    descriptionEl.textContent = solidity.description;
  }
}

function renderChips(root, values) {
  const container = getSlot(root, 'chips');
  if (!container) return;
  
  container.innerHTML = '';
  const fragment = document.createDocumentFragment();
  
  (values || []).forEach((chip) => {
    const chipElement = document.createElement('span');
    chipElement.className = 'requirement-chip';
    chipElement.textContent = chip.label || chip;
    fragment.appendChild(chipElement);
  });
  
  container.appendChild(fragment);
  
  // Handle empty state
  const emptyField = getField(root, 'chips-empty');
  if (emptyField) {
    if (values && values.length > 0) {
      emptyField.classList.add('d-none');
    } else {
      emptyField.classList.remove('d-none');
    }
  }
}

function renderBodySections(root, sections = []) {
  const container = getSlot(root, 'body-sections');
  if (!container) {
    return;
  }

  // Only render Notes section (Rationale is handled server-side)
  const notesSection = sections.find(s => s.title === 'Notes');
  
  if (notesSection) {
    const paragraph = document.createElement('p');
    paragraph.className = notesSection.empty ? 'mb-0 fst-italic' : 'mb-0';
    paragraph.textContent = notesSection.content;
    
    container.innerHTML = '';
    container.appendChild(paragraph);
  }
}

function renderRelationships(root, view, projectId) {
  const container = getSlot(root, 'relationships');
  if (!container) {
    return;
  }

  container.innerHTML = '';

  if (!view?.has_links) {
    const empty = document.createElement('p');
    empty.className = 'mb-0 text-muted';
    empty.textContent = 'No upstream or downstream relationships recorded.';
    container.appendChild(empty);
    return;
  }

  if (view.parent) {
    const wrapper = document.createElement('div');
    wrapper.className = 'mb-3';

    const label = document.createElement('div');
    label.className = 'small text-muted text-uppercase';
    label.textContent = 'Derived from';

    const link = document.createElement('a');
    link.className = 'fw-semibold';
    link.href = `/p/${projectId}/requirements/show/${view.parent.id}`;
    link.textContent = `${view.parent.reference} · ${view.parent.title}`;

    const status = document.createElement('span');
    status.className = 'badge bg-light text-dark border ms-2';
    status.textContent = view.parent.status;

    wrapper.append(label, link, status);
    container.appendChild(wrapper);
  }

  if (view.children?.length) {
    const label = document.createElement('div');
    label.className = 'small text-muted text-uppercase mb-2';
    label.textContent = 'Feeds';
    container.appendChild(label);

    const list = document.createElement('ul');
    list.className = 'list-unstyled mb-0';

    view.children.forEach((child) => {
      const item = document.createElement('li');
      item.className = 'd-flex align-items-center mb-2';

      const arrow = document.createElement('span');
      arrow.className = 'text-muted me-2';
      arrow.textContent = '→';

      const link = document.createElement('a');
      link.className = 'fw-semibold flex-grow-1';
      link.href = `/p/${projectId}/requirements/show/${child.id}`;
      link.textContent = `${child.reference} · ${child.title}`;

      const status = document.createElement('span');
      status.className = 'badge bg-light text-dark border ms-2';
      status.textContent = child.status;

      item.append(arrow, link, status);
      list.appendChild(item);
    });

    container.appendChild(list);
  }
}

function renderVerification(root, view, canonical) {
  renderBadge(getFields(root, 'verification-badge'), view.verification_badge);
  setText(getFields(root, 'verification-state'), view.verification_badge.state);
  setText(
    getField(root, 'verification-percent'),
    `${view.verification_summary.percent}%`,
  );

  const progress = getField(root, 'verification-progress');
  if (progress) {
    progress.style.width = `${view.verification_summary.percent}%`;
    progress.setAttribute(
      'aria-valuenow',
      String(view.verification_summary.percent),
    );
  }

  setText(
    getField(root, 'verification-passed'),
    `Passed ${view.verification_summary.passed}`,
  );
  setText(
    getField(root, 'verification-failed'),
    `Failed ${view.verification_summary.failed}`,
  );
  setText(
    getField(root, 'verification-pending'),
    `Pending ${view.verification_summary.pending}`,
  );

  const toolInfo = getField(root, 'verification-tool');
  if (toolInfo) {
    const toolName = view.verification_summary.tool;
    const lastChecked = view.verification_summary.last_checked;
    toolInfo.textContent = `Tool: ${toolName || '—'} · Last checked ${lastChecked || '—'}`;
  }

  const testsContainer = getSlot(root, 'linked-tests');
  const testsFallback = getField(root, 'linked-tests-empty');

  if (testsContainer) {
    testsContainer.innerHTML = '';
  }

  if (!view.linked_tests?.length) {
    if (testsFallback) {
      testsFallback.classList.remove('d-none');
    }
    return;
  }

  if (testsFallback) {
    testsFallback.classList.add('d-none');
  }

  view.linked_tests.forEach((test) => {
    const link = document.createElement('a');
    link.className =
      'list-group-item list-group-item-action d-flex justify-content-between align-items-start';
    link.href = `/p/${canonical.project_id}/tests/show/${test.test_id}`;

    const info = document.createElement('div');
    info.className = 'me-3';

    const name = document.createElement('div');
    name.className = 'fw-semibold';
    name.textContent = test.test_name;

    const description = document.createElement('div');
    description.className = 'small text-muted';
    description.textContent = test.test_description;

    info.append(name, description);

    const status = document.createElement('span');
    status.className = 'badge bg-light text-dark border';
    status.textContent = test.test_status;

    link.append(info, status);
    testsContainer.appendChild(link);
  });
}

function renderMetadata(root, metadata) {
  if (!metadata) {
    return;
  }

  toElementArray(getFields(root, 'author-initial')).forEach((el) => {
    el.textContent = metadata.author.initial || '?';
    el.classList.add('bg-primary', 'text-white');
    el.classList.remove('bg-light', 'text-muted', 'border');
  });

  setText(getFields(root, 'author-name'), metadata.author.name);
  setText(
    getFields(root, 'author-timestamp'),
    metadata.author.timestamp ? `Created ${metadata.author.timestamp}` : '',
  );

  toElementArray(getFields(root, 'reviewer-initial')).forEach((el) => {
    if (metadata.reviewer.assigned) {
      el.textContent = metadata.reviewer.initial ?? '–';
      el.classList.add('bg-secondary', 'text-white');
      el.classList.remove('bg-light', 'text-muted', 'border');
    } else {
      el.textContent = '–';
      el.classList.add('bg-light', 'text-muted', 'border');
      el.classList.remove('bg-secondary', 'text-white');
    }
  });

  const reviewerNames = toElementArray(getFields(root, 'reviewer-name'));
  const reviewerTimestamps = getFields(root, 'reviewer-timestamp');

  if (metadata.reviewer.assigned) {
    reviewerNames.forEach((el) => {
      el.textContent = metadata.reviewer.name ?? '';
      el.classList.remove('text-muted');
    });
    setText(
      reviewerTimestamps,
      metadata.reviewer.timestamp ? `Reviewed ${metadata.reviewer.timestamp}` : '',
    );
  } else {
    reviewerNames.forEach((el) => {
      el.textContent = 'Unassigned';
      el.classList.add('text-muted');
    });
    setText(reviewerTimestamps, '');
  }

  setText(getFields(root, 'metadata-updated'), metadata.updated);
  setText(getFields(root, 'metadata-deadline'), metadata.deadline);
  setText(getFields(root, 'metadata-version'), metadata.version);
}

function initRequirementDetailsToggle() {
  const toggle = document.querySelector('[data-action="toggle-requirement-details"]');
  if (!toggle) {
    return;
  }

  const targetSelector = toggle.getAttribute('data-bs-target');
  if (!targetSelector) {
    return;
  }

  const target = document.querySelector(targetSelector);
  if (!target) {
    return;
  }

  const setExpandedState = (expanded) => {
    toggle.textContent = expanded ? 'Collapse' : 'Expand';
    toggle.setAttribute('aria-expanded', String(expanded));
  };

  setExpandedState(target.classList.contains('show'));

  target.addEventListener('hidden.bs.collapse', () => setExpandedState(false));
  target.addEventListener('shown.bs.collapse', () => setExpandedState(true));
}

function initCopyRequirementLink() {
  const trigger = document.querySelector('[data-action="copy-requirement-link"]');
  if (!trigger) {
    return;
  }

  const originalLabel = trigger.textContent;

  const setStatus = (label) => {
    trigger.textContent = label;
    trigger.disabled = true;
    setTimeout(() => {
      trigger.textContent = originalLabel;
      trigger.disabled = false;
    }, 2000);
  };

  trigger.addEventListener('click', async () => {
    const relativeUrl = trigger.getAttribute('data-requirement-url');
    if (!relativeUrl) {
      return;
    }

    const absoluteUrl = new URL(relativeUrl, window.location.origin).toString();
    let didCopy = false;

    if (navigator.clipboard?.writeText) {
      try {
        await navigator.clipboard.writeText(absoluteUrl);
        didCopy = true;
      } catch (error) {
        console.warn('Clipboard API failed, falling back to execCommand', error);
      }
    }

    if (!didCopy) {
      const helper = document.createElement('textarea');
      helper.value = absoluteUrl;
      helper.setAttribute('readonly', '');
      helper.style.position = 'fixed';
      helper.style.opacity = '0';
      document.body.appendChild(helper);
      helper.select();
      try {
        didCopy = document.execCommand('copy');
      } catch (error) {
        console.error('Fallback copy failed', error);
      } finally {
        document.body.removeChild(helper);
      }
    }

    setStatus(didCopy ? 'Link copied' : 'Copy failed');

    const dropdownToggle = trigger.closest('.dropdown')?.querySelector('[data-bs-toggle="dropdown"]');
    if (dropdownToggle && typeof bootstrap !== 'undefined' && bootstrap.Dropdown) {
      const dropdownInstance = bootstrap.Dropdown.getInstance(dropdownToggle);
      if (dropdownInstance) {
        dropdownInstance.hide();
      }
    }
  });
}

function hydratePage(view, canonical) {
  const root = document.querySelector('[data-requirement-root]');
  if (!root) {
    return;
  }

  setText(getField(root, 'reference'), view.reference);
  setText(getField(root, 'title'), canonical.requirement?.req_title);

  renderBadge(getField(root, 'status-badge'), view.status_badge);
  renderSolidity(root, view.solidity);
  renderVerification(root, view, canonical);
  renderChips(root, view.chips);
  renderMetadata(root, view.metadata);
  renderBodySections(root, view.body_sections);
  renderRelationships(root, view.relationships, canonical.project_id);
}

export function init() {
  const canonical = parseCanonicalData();
  if (!canonical) {
    return;
  }

  const view = buildRequirementViewModel(canonical);
  if (!view) {
    return;
  }

  hydratePage(view, canonical);
  initRequirementDetailsToggle();
  initCopyRequirementLink();

  initDiffModal({
    triggerSelector: '[data-action="show-changes"]',
    modalSelector: '#changesModal',
    contentSelector: '#changesContent',
  });
}
