import { buildRequirementViewModel } from '../presenters/requirement.js';
import { verificationPercent, verificationBadge } from '../presenters/requirement.js';
import { initDiffModal } from '../modules/diffModal.js';
import { showRequirementDiff } from '../modules/requirementDiffModal.js';
import { showNotification } from '../modules/notifications.js';
import { postJson, jsonFetch } from '../core/net.js';

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

function testStatusVariant(statusLabel) {
  if (!statusLabel) return 'default';
  const s = String(statusLabel).toLowerCase();
  if (s.includes('pass')) return 'passed';
  if (s.includes('fail')) return 'failed';
  if (s.includes('pending')) return 'proposal';
  if (s.includes('progress')) return 'draft';
  return 'default';
}

/** Count passed/failed/pending from linked_tests by status title (matches backend logic). */
function countsFromLinkedTests(linkedTests) {
  let passed = 0;
  let failed = 0;
  let pending = 0;
  (linkedTests || []).forEach((t) => {
    const s = String(t.status_id || '').toLowerCase();
    if (s.includes('pass')) passed += 1;
    else if (s.includes('fail')) failed += 1;
    else pending += 1;
  });
  return { total: passed + failed + pending, passed, failed, pending };
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
    if (badge.tag_color) {
      el.style.backgroundColor = badge.tag_color;
      el.style.color = '#fff';
      el.style.borderColor = badge.tag_color;
    } else {
      el.style.backgroundColor = '';
      el.style.color = '';
      el.style.borderColor = '';
    }
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

function renderComments(root, view, canonical) {
  const listSlot = getSlot(root, 'comments-list');
  const lockedSlot = getSlot(root, 'comments-locked');
  const formSlot = getSlot(root, 'add-comment-form');
  if (!listSlot) return;

  listSlot.innerHTML = '';
  const comments = view?.comments ?? {};
  const items = comments.items ?? [];

  if (items.length > 0) {
    const ul = document.createElement('ul');
    ul.className = 'list-unstyled mb-0';
    items.forEach((c) => {
      const li = document.createElement('li');
      li.className = 'py-2 border-bottom border-light border-opacity-50';
      const author = document.createElement('div');
      author.className = 'small fw-medium';
      author.textContent = `${c.author_name ?? 'Unknown'} · ${c.created_at ?? ''}`;
      const body = document.createElement('div');
      body.className = 'text-break';
      body.textContent = c.body ?? '';
      li.append(author, body);
      ul.appendChild(li);
    });
    listSlot.appendChild(ul);
  } else {
    const empty = document.createElement('p');
    empty.className = 'mb-0 text-muted';
    empty.textContent = 'No comments yet.';
    listSlot.appendChild(empty);
  }

  const enabled = comments.enabled === true;
  const lockedReason = comments.locked_reason;
  if (lockedSlot) {
    if (lockedReason) {
      lockedSlot.style.display = 'block';
      const reasonEl = lockedSlot.querySelector('[data-field="comments-locked-reason"]');
      if (reasonEl) reasonEl.textContent = lockedReason;
    } else {
      lockedSlot.style.display = 'none';
    }
  }
  if (formSlot) {
    formSlot.style.display = enabled ? 'block' : 'none';
  }
}

function renderRelationships(root, view, projectId, linkedTests = []) {
  const container = getSlot(root, 'relationships');
  if (!container) {
    return;
  }

  container.innerHTML = '';

  const hasParentOrChildren = view?.has_links === true;
  const hasLinkedTests = Array.isArray(linkedTests) && linkedTests.length > 0;

  if (!hasParentOrChildren && !hasLinkedTests) {
    const empty = document.createElement('p');
    empty.className = 'mb-0 text-muted';
    empty.textContent = 'No upstream or downstream relationships recorded.';
    container.appendChild(empty);
    return;
  }

  if (view?.parent_links?.length > 0) {
    const label = document.createElement('div');
    label.className = 'small text-muted text-uppercase mb-2';
    label.textContent = 'Upstream';
    container.appendChild(label);

    const list = document.createElement('ul');
    list.className = 'list-unstyled mb-0';

    view.parent_links.forEach((pl) => {
      if (!pl.target) return;
      const item = document.createElement('li');
      item.className = 'd-flex align-items-center mb-2';

      const typeBadge = document.createElement('span');
      typeBadge.className = 'badge bg-secondary me-2';
      typeBadge.textContent = pl.link_type;

      const link = document.createElement('a');
      link.className = 'fw-semibold flex-grow-1';
      link.href = `/p/${projectId}/requirements/show/${pl.target.id}`;
      link.textContent = `${pl.target.reference} · ${pl.target.title}`;

      item.append(typeBadge, link);
      list.appendChild(item);
    });

    container.appendChild(list);
  } else if (view?.parent) {
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

  // List all tests linked to this requirement (traceability req ↔ test)
  if (hasLinkedTests) {
    const label = document.createElement('div');
    label.className = 'small text-muted text-uppercase mb-2 mt-3';
    label.textContent = 'Verified by';
    container.appendChild(label);

    const list = document.createElement('ul');
    list.className = 'list-unstyled mb-0';

    linkedTests.forEach((test) => {
      const item = document.createElement('li');
      item.className = 'd-flex align-items-center mb-2';

      const link = document.createElement('a');
      link.className = 'fw-semibold';
      link.href = `/p/${projectId}/verifications/show/${test.id}`;
      link.textContent = `${test.reference_code || `Test #${test.id}`} · ${test.name || ''}`.trim() || `Test #${test.id}`;

      item.appendChild(link);
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

  const projectId = canonical.project_id;
  const statuses = canonical.test_statuses || [];
  const updateVerificationSummary = () => {
    const counts = countsFromLinkedTests(view.linked_tests);
    const percent = verificationPercent(counts);
    const badge = verificationBadge(counts, view.verification_badge?.label);
    setText(getField(root, 'verification-percent'), `${percent}%`);
    setText(getField(root, 'verification-passed'), `Passed ${counts.passed}`);
    setText(getField(root, 'verification-failed'), `Failed ${counts.failed}`);
    setText(getField(root, 'verification-pending'), `Pending ${counts.pending}`);
    setText(getFields(root, 'verification-state'), badge.state);
    renderBadge(getFields(root, 'verification-badge'), badge);
    const progress = getField(root, 'verification-progress');
    if (progress) {
      progress.style.width = `${percent}%`;
      progress.setAttribute('aria-valuenow', String(percent));
    }
  };

  view.linked_tests.forEach((test) => {
    const row = document.createElement('div');
    row.className =
      'list-group-item list-group-item-action d-flex justify-content-between align-items-start';
    row.setAttribute('data-test-id', String(test.id));
    row.setAttribute('data-status-id', String(test.test_status_id ?? test.status_id ?? ''));

    const link = document.createElement('a');
    link.href = `/p/${projectId}/verifications/show/${test.id}`;
    link.className = 'text-decoration-none text-body flex-grow-1 me-2';
    link.style.minWidth = '0';

    const info = document.createElement('div');
    info.className = 'me-2';

    const name = document.createElement('div');
    name.className = 'fw-semibold';
    name.textContent = test.name;

    const description = document.createElement('div');
    description.className = 'small text-muted';
    description.textContent = test.description || '';

    info.append(name, description);
    link.append(info);
    row.appendChild(link);

    const statusWrap = document.createElement('div');
    statusWrap.className = 'marreq-linked-test-status flex-shrink-0';
    statusWrap.setAttribute('data-inline-edit', 'status');
    const displayEl = document.createElement('span');
    displayEl.className = `badge border marreq-requirements-status-badge marreq-requirements-status-badge--${testStatusVariant(test.status_id)}`;
    displayEl.textContent = test.status_id || '—';
    displayEl.title = 'Click to change';
    displayEl.setAttribute('role', 'button');
    displayEl.setAttribute('tabindex', '0');
    displayEl.setAttribute('aria-label', 'Change test status');
    statusWrap.appendChild(displayEl);
    row.appendChild(statusWrap);

    if (statuses.length > 0) {
      displayEl.addEventListener('click', (e) => {
        e.preventDefault();
        e.stopPropagation();
        if (statusWrap.querySelector('.marreq-inline-edit-select')) return;
        const currentStatusId = parseInt(row.getAttribute('data-status-id'), 10) || 0;
        const select = document.createElement('select');
        select.className = 'marreq-inline-edit-select form-select form-select-sm';
        select.setAttribute('aria-label', 'Change status');
        statuses.forEach((s) => {
          const sid = typeof s.id === 'number' ? s.id : parseInt(s.id, 10);
          select.appendChild(
            new Option(s.title, String(sid), false, sid === currentStatusId)
          );
        });
        const initialValue = select.value;
        let applied = false;
        const apply = async () => {
          if (applied) return;
          const v = parseInt(select.value, 10);
          if (Number.isNaN(v)) return;
          const status = statuses.find(
            (s) => (typeof s.id === 'number' ? s.id : parseInt(s.id, 10)) === v
          );
          const displayText = status ? status.title : '—';
          applied = true;
          if (select.parentNode) select.remove();
          displayEl.hidden = false;
          try {
            await postJson(`/p/${projectId}/verifications/update-status/${test.id}`, { status_id: v });
            const variant = testStatusVariant(displayText);
            const tagColor = status?.tag_color || null;
            test.status_id = displayText;
            test.test_status_id = v;
            row.setAttribute('data-status-id', String(v));
            displayEl.textContent = displayText;
            displayEl.className = `badge border marreq-requirements-status-badge marreq-requirements-status-badge--${variant}`;
            if (tagColor) {
              displayEl.style.backgroundColor = tagColor;
              displayEl.style.color = '#fff';
              displayEl.style.borderColor = tagColor;
            } else {
              displayEl.style.backgroundColor = '';
              displayEl.style.color = '';
              displayEl.style.borderColor = '';
            }
            updateVerificationSummary();
            showNotification('Status updated successfully', 'success');
          } catch (err) {
            applied = false;
            const statusCode = err?.response?.status;
            const msg = err?.message || 'Update failed';
            const detail = statusCode ? ` (${statusCode})` : '';
            showNotification(msg + detail, 'error');
            console.error('Test status update failed:', err?.payload || err);
            window.location.reload();
          }
        };
        select.addEventListener('change', () => apply());
        select.addEventListener('blur', () => {
          if (applied) return;
          if (select.value !== initialValue) apply();
          else {
            if (select.parentNode) select.remove();
            displayEl.hidden = false;
          }
        });
        document.addEventListener('keydown', function esc(e) {
          if (e.key === 'Escape') {
            select.remove();
            displayEl.hidden = false;
            document.removeEventListener('keydown', esc);
          }
        });
        displayEl.hidden = true;
        statusWrap.appendChild(select);
        select.focus();
      });
      displayEl.addEventListener('keydown', (e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          displayEl.click();
        }
      });
    }

    testsContainer.appendChild(row);
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
  setText(getField(root, 'title'), canonical.requirement?.title);

  renderBadge(getField(root, 'status-badge'), view.status_badge);
  renderSolidity(root, view.solidity);
  renderVerification(root, view, canonical);
  renderChips(root, view.chips);
  renderMetadata(root, view.metadata);
  renderBodySections(root, view.body_sections);
  renderComments(root, view, canonical);
  renderRelationships(root, view.relationships, canonical.project_id, view.linked_tests);
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
  initCommentsForm(canonical);

  initDiffModal({
    triggerSelector: '[data-action="show-changes"]',
    modalSelector: '#changesModal',
    contentSelector: '#changesContent',
  });

  initVersionDiffHandler();
  initApprovalHandlers(canonical);
  initEditApprovedHandler(canonical);
}

function initVersionDiffHandler() {
  const root = document.querySelector('[data-requirement-root]');
  if (!root) return;
  root.addEventListener('click', async (e) => {
    const trigger = e.target.closest('[data-action="show-version-diff"]');
    if (!trigger) return;
    e.preventDefault();
    const reqId = trigger.getAttribute('data-req-id');
    const v1 = trigger.getAttribute('data-v1');
    const v2 = trigger.getAttribute('data-v2');
    if (!reqId || !v1 || !v2) return;
    try {
      const diff = await jsonFetch(
        `/api/requirements/${reqId}/versions/${v1}/diff/${v2}`,
        { credentials: 'same-origin' }
      );
      showRequirementDiff(diff);
    } catch (err) {
      const msg = err?.payload?.message || err?.message || 'Failed to load diff';
      showNotification(msg, 'error');
    }
  });
}

function initApprovalHandlers(canonical) {
  const section = document.querySelector('[data-approval-section]');
  if (!section) return;

  const reqId = canonical?.requirement?.id;
  const versionId = canonical?.current_version_id;
  if (reqId == null || versionId == null) return;

  const reviewedModal = document.getElementById('approvalReviewedModal');
  const approveModal = document.getElementById('approvalApproveModal');
  const reviewedConfirm = document.getElementById('approvalReviewedConfirm');
  const approveConfirm = document.getElementById('approvalApproveConfirm');

  section.addEventListener('click', (e) => {
    const btn = e.target.closest('[data-action="approval-transition"][data-state]');
    if (!btn) return;
    e.preventDefault();
    const state = btn.getAttribute('data-state');
    if (state === 'reviewed' && reviewedModal && typeof bootstrap !== 'undefined') {
      const modal = bootstrap.Modal.getOrCreateInstance(reviewedModal);
      reviewedConfirm.dataset.pendingState = state;
      modal.show();
    } else if (state === 'approved' && approveModal && typeof bootstrap !== 'undefined') {
      const modal = bootstrap.Modal.getOrCreateInstance(approveModal);
      approveConfirm.dataset.pendingState = state;
      modal.show();
    }
  });

  const doTransition = async (state) => {
    const url = `/api/requirements/${reqId}/versions/${versionId}/approval`;
    try {
      await jsonFetch(url, { method: 'PUT', body: { state } });
      showNotification(state === 'approved' ? 'Requirement approved.' : 'Marked as reviewed.', 'success');
      window.location.reload();
    } catch (err) {
      const msg = err?.payload?.message || err?.message || 'Request failed';
      showNotification(msg, 'error');
    }
  };

  if (reviewedConfirm) {
    reviewedConfirm.addEventListener('click', () => {
      const state = reviewedConfirm.dataset.pendingState || 'reviewed';
      if (reviewedModal && bootstrap.Modal) {
        bootstrap.Modal.getInstance(reviewedModal)?.hide();
      }
      doTransition(state);
    });
  }
  if (approveConfirm) {
    approveConfirm.addEventListener('click', () => {
      const state = approveConfirm.dataset.pendingState || 'approved';
      if (approveModal && bootstrap.Modal) {
        bootstrap.Modal.getInstance(approveModal)?.hide();
      }
      doTransition(state);
    });
  }
}

function initCommentsForm(canonical) {
  const form = document.querySelector('[data-action="add-comment"]');
  if (!form || !canonical?.requirement?.id) return;

  const reqId = canonical.requirement.id;
  const versionId = canonical.viewing_past_version ? canonical.viewing_version_id : canonical.current_version_id;

  form.addEventListener('submit', async (e) => {
    e.preventDefault();
    const textarea = form.querySelector('textarea[name="body"]');
    const body = textarea?.value?.trim();
    if (!body) return;

    const payload = { body };
    if (versionId != null) {
      payload.requirement_version_id = versionId;
    } else {
      payload.requirement_version_id = null;
    }

    try {
      await postJson(`/api/requirements/${reqId}/comments`, payload);
      showNotification('Comment added.', 'success');
      window.location.reload();
    } catch (err) {
      const msg = err?.payload?.message || err?.message || 'Failed to add comment';
      showNotification(msg, 'error');
    }
  });
}

function initEditApprovedHandler(canonical) {
  const link = document.querySelector('[data-action="edit-approved-requirement"]');
  if (!link) return;

  const modalEl = document.getElementById('editApprovedWarningModal');
  const confirmEl = document.getElementById('editApprovedConfirm');
  if (!modalEl || !confirmEl) return;

  link.addEventListener('click', (e) => {
    e.preventDefault();
    if (typeof bootstrap !== 'undefined') {
      const modal = bootstrap.Modal.getOrCreateInstance(modalEl);
      const href = link.getAttribute('data-edit-href');
      if (href) confirmEl.setAttribute('href', href);
      modal.show();
    } else {
      const href = link.getAttribute('data-edit-href');
      if (href) window.location.href = href;
    }
  });

  confirmEl.addEventListener('click', (e) => {
    e.preventDefault();
    const href = confirmEl.getAttribute('href');
    if (href) window.location.href = href;
  });
}
