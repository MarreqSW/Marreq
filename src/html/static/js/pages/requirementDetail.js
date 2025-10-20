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

function toElementArray(target) {
  if (!target) {
    return [];
  }
  if (typeof Element !== 'undefined' && target instanceof Element) {
    return [target];
  }
  return Array.from(target);
}

function getSlot(root, name) {
  return root.querySelector(`[data-slot="${name}"]`);
}

function setText(element, value) {
  toElementArray(element).forEach((el) => {
    if (!el) {
      return;
    }
    el.textContent = value ?? '';
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

function renderChips(root, chips = []) {
  const container = getSlot(root, 'chips');
  if (!container) {
    return;
  }

  const emptyMessage = getField(root, 'chips-empty');
  container.innerHTML = '';
  if (!chips.length) {
    if (emptyMessage) {
      emptyMessage.classList.remove('d-none');
    }
    return;
  }

  if (emptyMessage) {
    emptyMessage.classList.add('d-none');
  }

  chips.forEach((chip) => {
    const badge = document.createElement('span');
    badge.className = 'badge rounded-pill bg-light text-muted border';
    badge.textContent = chip.label;
    container.appendChild(badge);
  });
}

function renderBodySections(root, sections = []) {
  const container = getSlot(root, 'body-sections');
  if (!container) {
    return;
  }

  container.innerHTML = '';
  sections.forEach((section) => {
    const details = document.createElement('details');
    details.className = 'card shadow-sm requirement-section';
    details.open = true;

    const summary = document.createElement('summary');
    summary.className =
      'card-header bg-white d-flex justify-content-between align-items-center';

    const title = document.createElement('span');
    title.className = 'h5 mb-0';
    title.textContent = section.title;

    const body = document.createElement('div');
    body.className = 'card-body';

    const paragraph = document.createElement('p');
    paragraph.className = section.empty ? 'mb-0 text-muted' : 'mb-0';
    paragraph.textContent = section.content;

    summary.append(title);
    body.appendChild(paragraph);
    details.append(summary, body);
    container.appendChild(details);
  });
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

function renderAttachments(root, attachments = []) {
  const container = getSlot(root, 'attachments');
  if (!container) {
    return;
  }

  container.innerHTML = '';
  if (!attachments.length) {
    const empty = document.createElement('p');
    empty.className = 'mb-0 text-muted';
    empty.textContent = 'No supporting evidence has been linked yet.';
    container.appendChild(empty);
    return;
  }

  const list = document.createElement('div');
  list.className = 'list-group list-group-flush';

  attachments.forEach((attachment) => {
    const link = document.createElement('a');
    link.className =
      'list-group-item list-group-item-action d-flex justify-content-between align-items-center';
    link.href = attachment.href;
    link.target = '_blank';
    link.rel = 'noopener';

    const name = document.createElement('span');
    name.textContent = attachment.label;

    const icon = document.createElement('span');
    icon.className = 'small text-muted';
    icon.textContent = '↗';

    link.append(name, icon);
    list.appendChild(link);
  });

  container.appendChild(list);
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

function renderTimeline(root, timeline) {
  const container = getSlot(root, 'timeline');
  if (!container) {
    return;
  }

  container.innerHTML = '';
  timeline.forEach((entry) => {
    const item = document.createElement('li');
    item.className = 'requirement-timeline__item';
    if (entry.is_current) {
      item.classList.add('is-current');
    }

    const header = document.createElement('div');
    header.className =
      'requirement-timeline__header d-flex justify-content-between align-items-center flex-wrap gap-3';

    const version = document.createElement('span');
    version.className = 'badge bg-light text-dark border';
    version.textContent = entry.version;

    const timestamp = document.createElement('small');
    timestamp.className = 'text-muted';
    timestamp.textContent = entry.timestamp;

    header.append(version, timestamp);

    const summary = document.createElement('div');
    summary.className = 'fw-semibold mt-2 requirement-timeline__summary';
    summary.textContent = entry.summary;

    const actor = document.createElement('div');
    actor.className = 'small text-muted requirement-timeline__actor';
    actor.textContent = `by ${entry.actor || '—'}`;

    item.append(header, summary, actor);

    if (entry.old_values && entry.new_values) {
      const button = document.createElement('button');
      button.className = 'btn btn-sm btn-outline-secondary mt-2';
      button.type = 'button';
      button.setAttribute('data-action', 'show-changes');
      button.dataset.oldValues = JSON.stringify(entry.old_values);
      button.dataset.newValues = JSON.stringify(entry.new_values);
      button.textContent = 'View diff';
      item.appendChild(button);
    }

    container.appendChild(item);
  });
}

function renderComments(root, comments) {
  const container = getSlot(root, 'comments');
  const lockedMessage = getField(root, 'comments-locked');
  const emptyMessage = getField(root, 'comments-empty');
  const replyButton = getField(root, 'comments-reply-button');
  if (!container) {
    return;
  }

  container.innerHTML = '';

  if (!comments.enabled) {
    if (lockedMessage) {
      lockedMessage.textContent = comments.locked_reason ?? '';
      lockedMessage.classList.remove('d-none');
    }
    if (replyButton) {
      replyButton.classList.add('d-none');
    }
    return;
  }

  if (lockedMessage) {
    lockedMessage.classList.add('d-none');
  }

  if (!comments.has_items) {
    if (emptyMessage) {
      emptyMessage.classList.remove('d-none');
    }
    return;
  }

  if (emptyMessage) {
    emptyMessage.classList.add('d-none');
  }

  if (replyButton) {
    replyButton.classList.remove('d-none');
    replyButton.setAttribute('disabled', 'disabled');
  }

  comments.items.forEach((comment) => {
    const card = document.createElement('article');
    card.className = 'requirement-comment shadow-sm border rounded-3 p-3';

    const header = document.createElement('div');
    header.className =
      'd-flex justify-content-between align-items-center gap-3 mb-2 requirement-comment__header';

    const author = document.createElement('span');
    author.className = 'fw-semibold requirement-comment__author';
    author.textContent = comment.author ?? 'Unknown';

    const timestamp = document.createElement('small');
    timestamp.className = 'text-muted requirement-comment__timestamp';
    timestamp.textContent = comment.timestamp ?? '';

    header.append(author, timestamp);

    const body = document.createElement('p');
    body.className = 'mb-0 requirement-comment__body';
    body.textContent = comment.body ?? '';

    card.append(header, body);
    container.appendChild(card);
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
  renderAttachments(root, view.attachments);
  renderTimeline(root, view.timeline);
  renderComments(root, view.comments);
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
