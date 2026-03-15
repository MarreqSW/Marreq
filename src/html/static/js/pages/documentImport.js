import { deleteJson, jsonFetch, patchJson, postJson } from '../core/net.js';
import { showNotification } from '../modules/notifications.js';

function escapeHtml(value) {
  const div = document.createElement('div');
  div.textContent = value == null ? '' : String(value);
  return div.innerHTML;
}

function optionMarkup(items, currentValue, labelKey = 'title', blankLabel = 'Select…') {
  const normalized = currentValue == null ? '' : String(currentValue);
  const options = [`<option value="">${escapeHtml(blankLabel)}</option>`];
  items.forEach((item) => {
    const value = String(item.id);
    const selected = value === normalized ? ' selected' : '';
    options.push(`<option value="${escapeHtml(value)}"${selected}>${escapeHtml(item[labelKey] || '')}</option>`);
  });
  return options.join('');
}

function multiSelectMarkup(items, selectedIds) {
  const selected = new Set((selectedIds || []).map((id) => String(id)));
  return items
    .map((item) => {
      const value = String(item.id);
      const isSelected = selected.has(value) ? ' selected' : '';
      return `<option value="${escapeHtml(value)}"${isSelected}>${escapeHtml(item.title || '')}</option>`;
    })
    .join('');
}

function issueBadges(issues) {
  const blockerCount = issues.filter((issue) => issue.severity === 'blocker').length;
  const warningCount = issues.filter((issue) => issue.severity === 'warning').length;
  const badges = [];
  if (blockerCount > 0) {
    badges.push(`<span class="marreq-doc-import__badge marreq-doc-import__badge--blocker">${blockerCount} blocker${blockerCount > 1 ? 's' : ''}</span>`);
  }
  if (warningCount > 0) {
    badges.push(`<span class="marreq-doc-import__badge marreq-doc-import__badge--warning">${warningCount} warning${warningCount > 1 ? 's' : ''}</span>`);
  }
  return badges.join('');
}

function duplicateMarkup(suggestions) {
  if (!suggestions || suggestions.length === 0) {
    return '';
  }
  const items = suggestions
    .slice(0, 3)
    .map((suggestion) => `<li>${escapeHtml(suggestion.reference_code)} · ${escapeHtml(suggestion.title)}${suggestion.score ? ` (${Number(suggestion.score).toFixed(2)})` : ''}</li>`)
    .join('');
  return `
    <div class="marreq-doc-import__lineage">
      <strong>Possible matches</strong>
      <ul class="marreq-doc-import__issues">${items}</ul>
    </div>
  `;
}

function issueListMarkup(issues) {
  if (!issues || issues.length === 0) {
    return '<p class="marreq-doc-import__empty">No issues detected.</p>';
  }
  return `<ul class="marreq-doc-import__issues">${issues.map((issue) => `<li>${escapeHtml(issue.message)}</li>`).join('')}</ul>`;
}

function customFieldMap(candidate) {
  const map = new Map();
  (candidate.custom_fields || []).forEach((field) => {
    map.set(String(field.field_id), field.value || '');
  });
  return map;
}

function renderDefaults(session) {
  const defaults = session.review_state?.defaults || {};
  return `
    <div class="marreq-doc-import__candidate-fields">
      <div>
        <label class="marreq-doc-import__field-label" for="import-default-reviewer">Reviewer</label>
        <select id="import-default-reviewer" class="marreq-doc-import__field-select" data-default-field="reviewer_id">
          ${optionMarkup(session.lookups.users || [], defaults.reviewer_id, 'name', 'Select reviewer…')}
        </select>
      </div>
      <div>
        <label class="marreq-doc-import__field-label" for="import-default-category">Category</label>
        <select id="import-default-category" class="marreq-doc-import__field-select" data-default-field="category_id">
          ${optionMarkup(session.lookups.categories || [], defaults.category_id, 'title', 'No default')}
        </select>
      </div>
      <div>
        <label class="marreq-doc-import__field-label" for="import-default-applicability">Applicability</label>
        <select id="import-default-applicability" class="marreq-doc-import__field-select" data-default-field="applicability_id">
          ${optionMarkup(session.lookups.applicability || [], defaults.applicability_id, 'title', 'No default')}
        </select>
      </div>
      <div>
        <label class="marreq-doc-import__field-label" for="import-default-verification-status">Verification status</label>
        <select id="import-default-verification-status" class="marreq-doc-import__field-select" data-default-field="verification_status_id">
          ${optionMarkup(session.lookups.verification_statuses || [], defaults.verification_status_id, 'title', 'Select status…')}
        </select>
      </div>
      <div class="marreq-doc-import__field--wide">
        <label class="marreq-doc-import__field-label" for="import-default-source">Verification source</label>
        <input id="import-default-source" type="text" class="marreq-doc-import__field-input" data-default-field="verification_source"
          value="${escapeHtml(defaults.verification_source || '')}">
      </div>
    </div>
  `;
}

function renderSummary(session) {
  const summary = session.summary || {};
  return `
    <div class="marreq-doc-import__summary-card">
      <span class="marreq-doc-import__summary-label">Requirements</span>
      <span class="marreq-doc-import__summary-value">${summary.requirement_candidates || 0}</span>
    </div>
    <div class="marreq-doc-import__summary-card">
      <span class="marreq-doc-import__summary-label">Verifications</span>
      <span class="marreq-doc-import__summary-value">${summary.verification_candidates || 0}</span>
    </div>
    <div class="marreq-doc-import__summary-card">
      <span class="marreq-doc-import__summary-label">Trace Links</span>
      <span class="marreq-doc-import__summary-value">${summary.trace_link_candidates || 0}</span>
    </div>
    <div class="marreq-doc-import__summary-card">
      <span class="marreq-doc-import__summary-label">Blockers</span>
      <span class="marreq-doc-import__summary-value">${summary.blockers || 0}</span>
    </div>
    <div class="marreq-doc-import__summary-card">
      <span class="marreq-doc-import__summary-label">Warnings</span>
      <span class="marreq-doc-import__summary-value">${summary.warnings || 0}</span>
    </div>
  `;
}

function renderRequirementCandidate(session, candidate) {
  const customValues = customFieldMap(candidate);
  const customFields = (session.lookups.custom_fields || []).map((field) => `
    <div>
      <label class="marreq-doc-import__field-label" for="req-${escapeHtml(candidate.id)}-cf-${field.id}">${escapeHtml(field.label)}</label>
      <input id="req-${escapeHtml(candidate.id)}-cf-${field.id}" type="text" class="marreq-doc-import__field-input"
        data-candidate-kind="requirement" data-candidate-id="${escapeHtml(candidate.id)}"
        data-field-name="custom_field" data-custom-field-id="${field.id}" value="${escapeHtml(customValues.get(String(field.id)) || '')}">
    </div>
  `).join('');

  return `
    <article class="marreq-doc-import__candidate" data-candidate-kind="requirement" data-candidate-id="${escapeHtml(candidate.id)}">
      <div class="marreq-doc-import__candidate-head">
        <div>
          <label class="marreq-doc-import__checkbox">
            <input type="checkbox" data-field-name="include" ${candidate.include ? 'checked' : ''}>
            <span>Include requirement candidate</span>
          </label>
          <div class="marreq-doc-import__candidate-meta">
            <span class="marreq-doc-import__badge">Confidence ${Number(candidate.confidence || 0).toFixed(2)}</span>
            ${issueBadges(candidate.issues || [])}
          </div>
        </div>
      </div>
      <div class="marreq-doc-import__candidate-fields">
        <div class="marreq-doc-import__field--wide">
          <label class="marreq-doc-import__field-label">Title</label>
          <input type="text" class="marreq-doc-import__field-input" data-field-name="title" value="${escapeHtml(candidate.title || '')}">
        </div>
        <div>
          <label class="marreq-doc-import__field-label">Reference</label>
          <input type="text" class="marreq-doc-import__field-input" data-field-name="reference_code" value="${escapeHtml(candidate.reference_code || '')}">
        </div>
        <div>
          <label class="marreq-doc-import__field-label">Reviewer</label>
          <select class="marreq-doc-import__field-select" data-field-name="reviewer_id">
            ${optionMarkup(session.lookups.users || [], candidate.reviewer_id, 'name', 'Use default')}
          </select>
        </div>
        <div>
          <label class="marreq-doc-import__field-label">Category</label>
          <select class="marreq-doc-import__field-select" data-field-name="category_id">
            ${optionMarkup(session.lookups.categories || [], candidate.category_id, 'title', 'Use default')}
          </select>
        </div>
        <div>
          <label class="marreq-doc-import__field-label">Applicability</label>
          <select class="marreq-doc-import__field-select" data-field-name="applicability_id">
            ${optionMarkup(session.lookups.applicability || [], candidate.applicability_id, 'title', 'Use default')}
          </select>
        </div>
        <div class="marreq-doc-import__field--wide">
          <label class="marreq-doc-import__field-label">Verification methods</label>
          <select multiple class="marreq-doc-import__field-select" data-field-name="verification_method_ids">
            ${multiSelectMarkup(session.lookups.verification_methods || [], candidate.verification_method_ids || [])}
          </select>
        </div>
        <div class="marreq-doc-import__field--wide">
          <label class="marreq-doc-import__field-label">Statement</label>
          <textarea class="marreq-doc-import__field-textarea" data-field-name="description">${escapeHtml(candidate.description || '')}</textarea>
        </div>
        ${customFields}
      </div>
      ${duplicateMarkup(candidate.duplicate_suggestions)}
      <div class="marreq-doc-import__lineage"><strong>Lineage</strong>: ${escapeHtml(candidate.lineage_preview || '')}</div>
      ${issueListMarkup(candidate.issues || [])}
    </article>
  `;
}

function renderVerificationCandidate(session, candidate) {
  return `
    <article class="marreq-doc-import__candidate" data-candidate-kind="verification" data-candidate-id="${escapeHtml(candidate.id)}">
      <div class="marreq-doc-import__candidate-head">
        <div>
          <label class="marreq-doc-import__checkbox">
            <input type="checkbox" data-field-name="include" ${candidate.include ? 'checked' : ''}>
            <span>Include verification candidate</span>
          </label>
          <div class="marreq-doc-import__candidate-meta">
            <span class="marreq-doc-import__badge">Confidence ${Number(candidate.confidence || 0).toFixed(2)}</span>
            ${issueBadges(candidate.issues || [])}
          </div>
        </div>
      </div>
      <div class="marreq-doc-import__candidate-fields">
        <div class="marreq-doc-import__field--wide">
          <label class="marreq-doc-import__field-label">Name</label>
          <input type="text" class="marreq-doc-import__field-input" data-field-name="name" value="${escapeHtml(candidate.name || '')}">
        </div>
        <div>
          <label class="marreq-doc-import__field-label">Reference</label>
          <input type="text" class="marreq-doc-import__field-input" data-field-name="reference_code" value="${escapeHtml(candidate.reference_code || '')}">
        </div>
        <div>
          <label class="marreq-doc-import__field-label">Status</label>
          <select class="marreq-doc-import__field-select" data-field-name="status_id">
            ${optionMarkup(session.lookups.verification_statuses || [], candidate.status_id, 'title', 'Use default')}
          </select>
        </div>
        <div>
          <label class="marreq-doc-import__field-label">Verification method</label>
          <select class="marreq-doc-import__field-select" data-field-name="verification_method_id">
            ${optionMarkup(session.lookups.verification_methods || [], candidate.verification_method_id, 'title', 'None')}
          </select>
        </div>
        <div class="marreq-doc-import__field--wide">
          <label class="marreq-doc-import__field-label">Source</label>
          <input type="text" class="marreq-doc-import__field-input" data-field-name="source" value="${escapeHtml(candidate.source || '')}">
        </div>
        <div class="marreq-doc-import__field--wide">
          <label class="marreq-doc-import__field-label">Description</label>
          <textarea class="marreq-doc-import__field-textarea" data-field-name="description">${escapeHtml(candidate.description || '')}</textarea>
        </div>
      </div>
      ${duplicateMarkup(candidate.duplicate_suggestions)}
      <div class="marreq-doc-import__lineage"><strong>Lineage</strong>: ${escapeHtml(candidate.lineage_preview || '')}</div>
      ${issueListMarkup(candidate.issues || [])}
    </article>
  `;
}

function renderTraceLinkCandidate(candidate) {
  return `
    <article class="marreq-doc-import__candidate" data-candidate-kind="trace_link" data-candidate-id="${escapeHtml(candidate.id)}">
      <div class="marreq-doc-import__candidate-head">
        <div>
          <label class="marreq-doc-import__checkbox">
            <input type="checkbox" data-field-name="include" ${candidate.include ? 'checked' : ''}>
            <span>Include trace link candidate</span>
          </label>
          <div class="marreq-doc-import__candidate-meta">${issueBadges(candidate.issues || [])}</div>
        </div>
      </div>
      <div class="marreq-doc-import__candidate-fields">
        <div>
          <label class="marreq-doc-import__field-label">Requirement reference</label>
          <input type="text" class="marreq-doc-import__field-input" data-field-name="requirement_reference_code" value="${escapeHtml(candidate.requirement_reference_code || '')}">
        </div>
        <div>
          <label class="marreq-doc-import__field-label">Verification reference</label>
          <input type="text" class="marreq-doc-import__field-input" data-field-name="verification_reference_code" value="${escapeHtml(candidate.verification_reference_code || '')}">
        </div>
      </div>
      <div class="marreq-doc-import__lineage"><strong>Lineage</strong>: ${escapeHtml(candidate.lineage_preview || '')}</div>
      ${issueListMarkup(candidate.issues || [])}
    </article>
  `;
}

function renderRequirementLinkCandidate(session, candidate) {
  return `
    <article class="marreq-doc-import__candidate" data-candidate-kind="requirement_link" data-candidate-id="${escapeHtml(candidate.id)}">
      <div class="marreq-doc-import__candidate-head">
        <div>
          <label class="marreq-doc-import__checkbox">
            <input type="checkbox" data-field-name="include" ${candidate.include ? 'checked' : ''}>
            <span>Include requirement link candidate</span>
          </label>
          <div class="marreq-doc-import__candidate-meta">${issueBadges(candidate.issues || [])}</div>
        </div>
      </div>
      <div class="marreq-doc-import__candidate-fields">
        <div>
          <label class="marreq-doc-import__field-label">Source requirement</label>
          <input type="text" class="marreq-doc-import__field-input" data-field-name="source_requirement_reference_code" value="${escapeHtml(candidate.source_requirement_reference_code || '')}">
        </div>
        <div>
          <label class="marreq-doc-import__field-label">Target requirement</label>
          <input type="text" class="marreq-doc-import__field-input" data-field-name="target_requirement_reference_code" value="${escapeHtml(candidate.target_requirement_reference_code || '')}">
        </div>
        <div>
          <label class="marreq-doc-import__field-label">Link type</label>
          <select class="marreq-doc-import__field-select" data-field-name="link_type">
            <option value="">Select link type…</option>
            ${(session.lookups.requirement_link_types || []).map((type) => {
              const selected = type === candidate.link_type ? ' selected' : '';
              return `<option value="${escapeHtml(type)}"${selected}>${escapeHtml(type)}</option>`;
            }).join('')}
          </select>
        </div>
        <div class="marreq-doc-import__field--wide">
          <label class="marreq-doc-import__field-label">Rationale</label>
          <textarea class="marreq-doc-import__field-textarea" data-field-name="rationale">${escapeHtml(candidate.rationale || '')}</textarea>
        </div>
      </div>
      <div class="marreq-doc-import__lineage"><strong>Lineage</strong>: ${escapeHtml(candidate.lineage_preview || '')}</div>
      ${issueListMarkup(candidate.issues || [])}
    </article>
  `;
}

function renderDiagnostics(session) {
  const blockers = session.diagnostics?.blockers || [];
  const warnings = session.diagnostics?.warnings || [];
  const parts = [];
  if (blockers.length > 0) {
    parts.push(`<div><span class="marreq-doc-import__badge marreq-doc-import__badge--blocker">${blockers.length} blockers</span>${issueListMarkup(blockers)}</div>`);
  }
  if (warnings.length > 0) {
    parts.push(`<div><span class="marreq-doc-import__badge marreq-doc-import__badge--warning">${warnings.length} warnings</span>${issueListMarkup(warnings)}</div>`);
  }
  if (parts.length === 0) {
    return '<p class="marreq-doc-import__empty">No open blockers or warnings.</p>';
  }
  return parts.join('');
}

function renderCandidateList(items, renderer, emptyMessage) {
  if (!items || items.length === 0) {
    return `<p class="marreq-doc-import__empty">${escapeHtml(emptyMessage)}</p>`;
  }
  return items.map(renderer).join('');
}

function renderSession(root, session) {
  root.querySelector('[data-role="summary"]').innerHTML = renderSummary(session);
  root.querySelector('[data-role="defaults"]').innerHTML = renderDefaults(session);
  root.querySelector('[data-role="diagnostics"]').innerHTML = renderDiagnostics(session);
  root.querySelector('[data-role="requirements"]').innerHTML = renderCandidateList(
    session.candidates?.requirements,
    (candidate) => renderRequirementCandidate(session, candidate),
    'No requirement candidates detected.',
  );
  root.querySelector('[data-role="verifications"]').innerHTML = renderCandidateList(
    session.candidates?.verifications,
    (candidate) => renderVerificationCandidate(session, candidate),
    'No verification candidates detected.',
  );
  root.querySelector('[data-role="trace-links"]').innerHTML = renderCandidateList(
    session.candidates?.trace_links,
    (candidate) => renderTraceLinkCandidate(candidate),
    'No trace link candidates detected.',
  );
  root.querySelector('[data-role="requirement-links"]').innerHTML = renderCandidateList(
    session.candidates?.requirement_links,
    (candidate) => renderRequirementLinkCandidate(session, candidate),
    'No requirement link candidates detected.',
  );
  root.dataset.readyToCommit = session.summary?.ready_to_commit ? 'true' : 'false';
  syncCommitState(root);
}

function integerValue(value) {
  const numeric = Number.parseInt(value, 10);
  return Number.isFinite(numeric) && numeric > 0 ? numeric : 0;
}

export function serializeReviewForm(root) {
  const defaults = {};
  root.querySelectorAll('[data-default-field]').forEach((input) => {
    const field = input.dataset.defaultField;
    if (input.tagName === 'SELECT') {
      defaults[field] = integerValue(input.value) || null;
    } else {
      defaults[field] = input.value.trim() || null;
    }
  });

  const requirements = Array.from(root.querySelectorAll('[data-candidate-kind="requirement"]')).map((node) => {
    const customFields = Array.from(node.querySelectorAll('[data-field-name="custom_field"]')).map((input) => ({
      field_id: integerValue(input.dataset.customFieldId),
      value: input.value.trim() || null,
    }));
    return {
      candidate_id: node.dataset.candidateId,
      include: node.querySelector('[data-field-name="include"]').checked,
      title: node.querySelector('[data-field-name="title"]').value.trim(),
      description: node.querySelector('[data-field-name="description"]').value.trim(),
      reference_code: node.querySelector('[data-field-name="reference_code"]').value.trim(),
      reviewer_id: integerValue(node.querySelector('[data-field-name="reviewer_id"]').value) || null,
      category_id: integerValue(node.querySelector('[data-field-name="category_id"]').value) || null,
      applicability_id: integerValue(node.querySelector('[data-field-name="applicability_id"]').value) || null,
      verification_method_ids: Array.from(node.querySelector('[data-field-name="verification_method_ids"]').selectedOptions).map((option) => integerValue(option.value)).filter(Boolean),
      custom_fields: customFields,
    };
  });

  const verifications = Array.from(root.querySelectorAll('[data-candidate-kind="verification"]')).map((node) => ({
    candidate_id: node.dataset.candidateId,
    include: node.querySelector('[data-field-name="include"]').checked,
    name: node.querySelector('[data-field-name="name"]').value.trim(),
    description: node.querySelector('[data-field-name="description"]').value.trim(),
    reference_code: node.querySelector('[data-field-name="reference_code"]').value.trim(),
    source: node.querySelector('[data-field-name="source"]').value.trim(),
    status_id: integerValue(node.querySelector('[data-field-name="status_id"]').value) || null,
    verification_method_id: integerValue(node.querySelector('[data-field-name="verification_method_id"]').value) || null,
  }));

  const trace_links = Array.from(root.querySelectorAll('[data-candidate-kind="trace_link"]')).map((node) => ({
    candidate_id: node.dataset.candidateId,
    include: node.querySelector('[data-field-name="include"]').checked,
    requirement_reference_code: node.querySelector('[data-field-name="requirement_reference_code"]').value.trim(),
    verification_reference_code: node.querySelector('[data-field-name="verification_reference_code"]').value.trim(),
  }));

  const requirement_links = Array.from(root.querySelectorAll('[data-candidate-kind="requirement_link"]')).map((node) => ({
    candidate_id: node.dataset.candidateId,
    include: node.querySelector('[data-field-name="include"]').checked,
    source_requirement_reference_code: node.querySelector('[data-field-name="source_requirement_reference_code"]').value.trim(),
    target_requirement_reference_code: node.querySelector('[data-field-name="target_requirement_reference_code"]').value.trim(),
    link_type: node.querySelector('[data-field-name="link_type"]').value.trim() || null,
    rationale: node.querySelector('[data-field-name="rationale"]').value.trim() || null,
  }));

  return {
    defaults,
    requirements,
    verifications,
    trace_links,
    requirement_links,
  };
}

export function syncCommitState(root) {
  const commitButton = root.querySelector('[data-role="commit-review"]');
  const confirmInput = root.querySelector('[data-role="confirm-commit"]');
  if (!commitButton || !confirmInput) {
    return;
  }

  const isReady = root.dataset.readyToCommit === 'true';
  commitButton.disabled = !(isReady && confirmInput.checked);
}

async function saveSession(root) {
  const projectId = root.dataset.projectId;
  const sessionId = root.dataset.sessionId;
  const patch = serializeReviewForm(root);
  return patchJson(`/api/projects/${projectId}/document_imports/${sessionId}`, patch);
}

function setError(root, message) {
  const node = root.querySelector('[data-role="review-error"]');
  if (!node) {
    return;
  }
  if (!message) {
    node.hidden = true;
    node.textContent = '';
    return;
  }
  node.hidden = false;
  node.textContent = message;
}

async function loadSession(root) {
  const projectId = root.dataset.projectId;
  const sessionId = root.dataset.sessionId;
  const session = await jsonFetch(`/api/projects/${projectId}/document_imports/${sessionId}`);
  renderSession(root, session);
  return session;
}

export async function initReviewPage(root) {
  let currentSession = await loadSession(root);
  root.querySelector('[data-role="confirm-commit"]').addEventListener('change', () => {
    syncCommitState(root);
  });

  root.querySelector('[data-role="save-review"]').addEventListener('click', async () => {
    try {
      currentSession = await saveSession(root);
      renderSession(root, currentSession);
      setError(root, '');
      showNotification('Review changes saved.', 'success');
    } catch (error) {
      setError(root, error.message);
    }
  });

  root.querySelector('[data-role="commit-review"]').addEventListener('click', async () => {
    const confirmed = root.querySelector('[data-role="confirm-commit"]').checked;
    if (!confirmed) {
      setError(root, 'Check the confirmation box before committing the import.');
      return;
    }

    try {
      currentSession = await saveSession(root);
      renderSession(root, currentSession);
      const response = await postJson(
        `/api/projects/${root.dataset.projectId}/document_imports/${root.dataset.sessionId}/commit`,
        { confirm: true },
      );
      showNotification('Document import committed successfully.', 'success');
      window.location.href = `/p/${root.dataset.projectId}/requirements`;
      return response;
    } catch (error) {
      setError(root, error.message);
    }
  });

  root.querySelector('[data-role="discard-review"]').addEventListener('click', async () => {
    if (!window.confirm('Discard this dry-run import session?')) {
      return;
    }
    try {
      await deleteJson(`/api/projects/${root.dataset.projectId}/document_imports/${root.dataset.sessionId}`);
      window.location.href = `/p/${root.dataset.projectId}/import_document`;
    } catch (error) {
      setError(root, error.message);
    }
  });
}

export async function init() {
  const root = document.querySelector('[data-review-root]');
  if (!root) {
    return;
  }

  try {
    await initReviewPage(root);
  } catch (error) {
    setError(root, error.message);
  }
}
