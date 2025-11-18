export const EMPTY_MESSAGE = 'No content provided yet.';

function normalise(value) {
  return (value ?? '').toString().trim();
}

function safeNumber(value) {
  return Number.isFinite(value) ? value : 0;
}

export function statusBadge(statusLabel = '') {
  const label = normalise(statusLabel);
  switch (label.toLowerCase()) {
    case 'accepted':
    case 'finished':
      return { label, variant: 'bg-success' };
    case 'draft':
    case 'proposal':
      return { label, variant: 'bg-secondary' };
    case 'rejected':
    case 'cancelled':
      return { label, variant: 'bg-danger' };
    default:
      return { label, variant: 'bg-secondary' };
  }
}

export function verificationPercent(counts = {}) {
  const total = safeNumber(counts.total);
  const passed = safeNumber(counts.passed);
  if (total <= 0) {
    return 0;
  }
  return Math.round((passed / total) * 100);
}

export function verificationBadge(counts = {}, verificationLabel = '') {
  const total = safeNumber(counts.total);
  const failed = safeNumber(counts.failed);
  const pending = safeNumber(counts.pending);

  if (total === 0) {
    return {
      label: normalise(verificationLabel),
      variant: 'bg-warning',
      state: 'No verifications linked yet',
    };
  }

  if (failed === 0 && pending === 0) {
    return {
      label: normalise(verificationLabel),
      variant: 'bg-primary',
      state: 'All linked verifications are passing',
    };
  }

  if (failed === 0) {
    return {
      label: normalise(verificationLabel),
      variant: 'bg-info',
      state: 'Verification in progress',
    };
  }

  return {
    label: normalise(verificationLabel),
    variant: 'bg-danger',
    state: 'Verification needs attention',
  };
}

export function solidity(counts = {}, statusLabel = '') {
  const total = safeNumber(counts.total);
  const failed = safeNumber(counts.failed);
  const pending = safeNumber(counts.pending);
  const status = normalise(statusLabel);

  let label;
  if (total === 0) {
    label = status.toLowerCase() === 'draft' ? 'Needs definition' : 'Unverified';
  } else if (failed === 0 && pending === 0) {
    label = 'Rock solid';
  } else if (failed === 0) {
    label = 'Under evaluation';
  } else {
    label = 'At risk';
  }

  const variantMap = {
    'Rock solid': 'text-success',
    'Under evaluation': 'text-info',
    'At risk': 'text-danger',
  };

  const descriptionMap = {
    'Rock solid': 'All linked verifications have passed.',
    'Under evaluation': 'Waiting for pending verification results.',
    'At risk': 'At least one verification failed; needs attention.',
    'Needs definition': 'Draft requirement without verification evidence yet.',
    Unverified: 'No verification evidence linked yet.',
  };

  return {
    label,
    variant: variantMap[label] ?? 'text-muted',
    description: descriptionMap[label] ?? descriptionMap.Unverified,
  };
}

export function initials(name = '') {
  const trimmed = normalise(name);
  if (!trimmed) {
    return '?';
  }
  return trimmed[0].toUpperCase();
}

export function reference(entity = {}) {
  const ref = normalise(entity.reference_code);
  if (ref) {
    return ref;
  }
  const id = entity.id ?? '';
  return `REQ-${String(id).padStart(4, '0')}`;
}

export function purpose(description = '') {
  const text = normalise(description);
  if (!text) {
    return '';
  }

  const [firstParagraph] = text.split(/\n{2,}/);
  return normalise(firstParagraph);
}

export function notesAndAttachments(link = '') {
  const trimmed = normalise(link);
  if (!trimmed) {
    return {
      notes: 'No implementation notes recorded.',
      attachments: [],
    };
  }

  return {
    notes: `Primary reference available at ${trimmed}`,
    attachments: [
      {
        label: 'Supporting evidence',
        href: trimmed,
      },
    ],
  };
}

function formatDateTime(value) {
  const raw = normalise(value);
  if (!raw) {
    return '';
  }

  const parsed = new Date(raw);
  if (Number.isNaN(parsed.valueOf())) {
    return raw;
  }
  return parsed.toLocaleString();
}

function formatTimelineEntry(entry, index, totalVersions) {
  const summary =
    entry?.log?.description ??
    `${entry?.log?.action_type ?? 'Update'} requirement`;

  return {
    version: `v${Math.max(totalVersions - (index + 1), 1)}`,
    summary,
    actor: normalise(entry?.username),
    timestamp: formatDateTime(entry?.log?.created_at),
    action: entry?.log?.action_type ?? '',
    old_values: entry?.log?.old_values ?? null,
    new_values: entry?.log?.new_values ?? null,
    is_current: false,
  };
}

export function timeline({ requirement = {}, historyEntries = [] } = {}) {
  const updateDate = requirement.update_date;
  const reviewer = normalise(requirement.reviewer_id);
  const actor = reviewer || normalise(requirement.author_id);
  const totalVersions = historyEntries.length + 1;

  const entries = [
    {
      version: `v${totalVersions}`,
      summary: `Current revision — ${normalise(requirement.status_id)}`,
      actor,
      timestamp: updateDate,
      action: 'CURRENT',
      old_values: null,
      new_values: null,
      is_current: true,
    },
  ];

  historyEntries.forEach((entry, index) => {
    entries.push(formatTimelineEntry(entry, index, totalVersions));
  });

  return entries;
}

function relationships(rawRelationships = {}) {
  const format = (item) => {
    if (!item) {
      return null;
    }

    return {
      id: item.id,
      reference: reference(item),
      title: normalise(item.title),
      status: normalise(item.status_id ?? item.req_current_status_id),
    };
  };

  const parent = format(rawRelationships.parent);
  const children = (rawRelationships.children ?? [])
    .map(format)
    .filter(Boolean);

  return {
    parent,
    children,
    has_links: Boolean(parent) || children.length > 0,
  };
}

function commentsView(items = [], status = '') {
  const normalizedStatus = normalise(status).toLowerCase();
  const lockedStatuses = {
    accepted: 'Read-only: requirement accepted and locked',
    rejected: 'Archived requirement: comments are closed',
  };

  const lockedReason = lockedStatuses[normalizedStatus] ?? null;
  return {
    enabled: !lockedReason,
    items,
    has_items: items.length > 0,
    locked_reason: lockedReason,
  };
}

function makeSection(title, value, fallback = EMPTY_MESSAGE) {
  const content = normalise(value);
  if (content) {
    return { title, content, empty: false };
  }
  return { title, content: fallback, empty: true };
}

export function buildRequirementViewModel(canonical = {}) {
  if (!canonical || typeof canonical !== 'object') {
    return null;
  }

  const requirement = canonical.requirement ?? {};
  const counts = canonical.verification?.counts ?? {};
  const historyEntries = canonical.history?.entries ?? [];

  const badge = statusBadge(requirement.status_id);
  const verification = verificationBadge(counts, requirement.verification_method_id);
  const solidityView = solidity(counts, requirement.status_id);
  const percent = verificationPercent(counts);
  const rationale =
    normalise(requirement.justification) || 'No rationale documented yet.';
  const notesResult = notesAndAttachments(requirement.req_link);
  const relationshipsView = relationships(canonical.relationships);
  const timelineEntries = timeline({
    requirement,
    historyEntries,
  });

  const authorName = normalise(requirement.author_id);
  const reviewerName = normalise(requirement.reviewer_id);
  const reviewerAssigned = Boolean(reviewerName);

  const metadata = {
    author: {
      name: authorName,
      timestamp: requirement.creation_date,
      initial: initials(authorName),
    },
    reviewer: {
      name: reviewerName,
      timestamp: reviewerAssigned
        ? requirement.update_date
        : null,
      initial: reviewerAssigned ? initials(reviewerName) : null,
      assigned: reviewerAssigned,
    },
    updated: requirement.update_date,
    deadline: requirement.deadline_date,
    version: timelineEntries[0]?.version ?? 'v1',
  };

  const bodySections = [
    makeSection('Rationale', rationale),
    makeSection('Notes', notesResult.notes, notesResult.notes),
  ];

  return {
    reference: reference(requirement),
    status_badge: badge,
    verification_badge: verification,
    solidity: solidityView,
    chips: [
      { label: normalise(requirement.category_id), type: 'category' },
      { label: normalise(requirement.applicability_id), type: 'applicability' },
    ].filter((chip) => chip.label),
    metadata,
    body_sections: bodySections,
    relationships: relationshipsView,
    attachments: notesResult.attachments,
    verification_summary: {
      total: safeNumber(counts.total),
      passed: safeNumber(counts.passed),
      failed: safeNumber(counts.failed),
      pending: safeNumber(counts.pending),
      percent,
      last_checked: requirement.update_date,
      tool: normalise(canonical.verification?.tool_name || requirement.verification_method_id),
    },
    linked_tests: canonical.linked_tests ?? [],
    timeline: timelineEntries,
    comments: commentsView(canonical.comments?.items ?? [], requirement.status_id),
  };
}
