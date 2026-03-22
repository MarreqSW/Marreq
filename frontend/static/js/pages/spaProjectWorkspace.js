/**
 * Project workspace: overview, requirements list, tests (verifications), traceability matrix.
 * Data from JSON APIs (parity with legacy server-rendered templates).
 */
import { mountAppShell } from './spaHome.js';

const fetchOpts = {
  credentials: 'include',
  cache: 'no-store',
  headers: { Accept: 'application/json' },
};

function escapeHtml(s) {
  if (s == null) {
    return '';
  }
  return String(s)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

function escapeAttr(s) {
  return escapeHtml(s).replace(/'/g, '&#39;');
}

async function apiJson(url) {
  const res = await fetch(url, fetchOpts);
  const text = await res.text();
  if (!res.ok) {
    throw new Error(`${url} → HTTP ${res.status}: ${text.trim().slice(0, 240)}`);
  }
  if (!text) {
    return null;
  }
  return JSON.parse(text);
}

function renderProjectTabs(routeSlug, active) {
  const tabs = [
    ['home', 'Overview', `/${routeSlug}`],
    ['requirements', 'Requirements', `/${routeSlug}/requirements`],
    ['verifications', 'Tests', `/${routeSlug}/verifications`],
    ['matrix', 'Matrix', `/${routeSlug}/matrix`],
  ];
  const items = tabs
    .map(([key, label, href]) => {
      const isActive = active === key;
      return `<li class="nav-item">
        <a class="nav-link${isActive ? ' active' : ''}" href="${escapeAttr(href)}"${
          isActive ? ' aria-current="page"' : ''
        }>${escapeHtml(label)}</a>
      </li>`;
    })
    .join('');
  return `<ul class="nav nav-tabs mb-4 flex-wrap marreq-project-tabs">${items}</ul>`;
}

function renderWorkspaceHeader(routeSlug, meta, activeTab) {
  return `<header class="marreq-main__header">
  <button type="button" class="marreq-mobile-toggle" id="mobileToggle" aria-label="Toggle navigation">
    <i class="fas fa-bars" aria-hidden="true"></i>
  </button>
  <div class="marreq-main__title">
    <h1>${escapeHtml(meta.name)}</h1>
    <p class="marreq-main__subtitle">Project <span class="text-muted font-monospace">${escapeHtml(
      routeSlug,
    )}</span></p>
  </div>
</header>
${renderProjectTabs(routeSlug, activeTab)}`;
}

function renderProjectHome(routeSlug, meta) {
  const desc =
    meta.description != null && String(meta.description).trim() !== ''
      ? `<p class="lead text-muted">${escapeHtml(meta.description)}</p>`
      : '';
  return `${renderWorkspaceHeader(routeSlug, meta, 'home')}
<section class="marreq-section">
  ${desc}
  <p class="text-muted">Choose a view (legacy templates are replaced by this SPA + <code>/api</code>).</p>
  <div class="d-flex flex-wrap gap-2 mt-3">
    <a class="btn btn-primary" href="/${escapeAttr(routeSlug)}/requirements"><i class="fas fa-clipboard-list me-2" aria-hidden="true"></i>Requirements</a>
    <a class="btn btn-outline-primary" href="/${escapeAttr(routeSlug)}/verifications"><i class="fas fa-vial me-2" aria-hidden="true"></i>Tests</a>
    <a class="btn btn-outline-primary" href="/${escapeAttr(routeSlug)}/matrix"><i class="fas fa-table me-2" aria-hidden="true"></i>Traceability matrix</a>
  </div>
</section>`;
}

function renderRequirementsView(routeSlug, meta, statuses, categories, requirements) {
  const statusById = new Map(
    (statuses || []).filter((s) => s.project_id === meta.id).map((s) => [s.id, s.title]),
  );
  const catById = new Map(
    (categories || []).filter((c) => c.project_id === meta.id).map((c) => [c.id, c.title]),
  );

  const rows = (requirements || [])
    .map((r) => {
      const st = statusById.get(r.status_id) ?? `Status #${r.status_id}`;
      const cat = catById.get(r.category_id) ?? `Category #${r.category_id}`;
      return `<tr>
        <td><code>${escapeHtml(r.reference_code)}</code></td>
        <td>${escapeHtml(r.title)}</td>
        <td>${escapeHtml(st)}</td>
        <td>${escapeHtml(cat)}</td>
        <td><span class="badge text-bg-secondary">${escapeHtml(r.approval_state || '')}</span></td>
      </tr>`;
    })
    .join('');

  const table =
    requirements && requirements.length > 0
      ? `<div class="table-responsive">
  <table class="table table-hover table-sm align-middle marreq-table">
    <thead><tr><th>Ref</th><th>Title</th><th>Status</th><th>Category</th><th>Approval</th></tr></thead>
    <tbody>${rows}</tbody>
  </table>
</div>`
      : `<div class="marreq-empty-state py-5 text-center text-muted">
  <i class="fas fa-clipboard-list fa-2x mb-3" aria-hidden="true"></i>
  <p>No requirements in this project yet.</p>
</div>`;

  return `${renderWorkspaceHeader(routeSlug, meta, 'requirements')}
<section class="marreq-section">
  <div class="marreq-section__header">
    <h2 class="marreq-section__title">Requirements</h2>
    <span class="text-muted small">${requirements?.length ?? 0} items</span>
  </div>
  ${table}
</section>`;
}

function renderVerificationsView(routeSlug, meta, statuses, verifications) {
  const statusById = new Map((statuses || []).map((s) => [s.id, s.title]));

  const rows = (verifications || [])
    .map((v) => {
      const st = statusById.get(v.status_id) ?? `Status #${v.status_id}`;
      return `<tr>
        <td><code>${escapeHtml(v.reference_code)}</code></td>
        <td>${escapeHtml(v.name)}</td>
        <td>${escapeHtml(st)}</td>
        <td>${escapeHtml(v.source || '')}</td>
      </tr>`;
    })
    .join('');

  const table =
    verifications && verifications.length > 0
      ? `<div class="table-responsive">
  <table class="table table-hover table-sm align-middle marreq-table">
    <thead><tr><th>Ref</th><th>Name</th><th>Status</th><th>Source</th></tr></thead>
    <tbody>${rows}</tbody>
  </table>
</div>`
      : `<div class="marreq-empty-state py-5 text-center text-muted">
  <i class="fas fa-vial fa-2x mb-3" aria-hidden="true"></i>
  <p>No tests (verifications) in this project yet.</p>
</div>`;

  return `${renderWorkspaceHeader(routeSlug, meta, 'verifications')}
<section class="marreq-section">
  <div class="marreq-section__header">
    <h2 class="marreq-section__title">Tests (verifications)</h2>
    <span class="text-muted small">${verifications?.length ?? 0} items</span>
  </div>
  ${table}
</section>`;
}

function renderMatrixView(routeSlug, meta, links, reqById, verById) {
  const rows = (links || [])
    .map((m) => {
      const rref = reqById.get(m.req_id) ?? `#${m.req_id}`;
      const vname = verById.get(m.verification_id) ?? `#${m.verification_id}`;
      const suspect = m.suspect
        ? '<span class="badge text-bg-warning">Suspect</span>'
        : '<span class="badge text-bg-success">OK</span>';
      return `<tr>
        <td><code>${escapeHtml(rref)}</code></td>
        <td><code>${escapeHtml(vname)}</code></td>
        <td>${suspect}</td>
      </tr>`;
    })
    .join('');

  const table =
    links && links.length > 0
      ? `<div class="table-responsive">
  <table class="table table-hover table-sm align-middle marreq-table">
    <thead><tr><th>Requirement</th><th>Verification</th><th>Link</th></tr></thead>
    <tbody>${rows}</tbody>
  </table>
</div>`
      : `<div class="marreq-empty-state py-5 text-center text-muted">
  <i class="fas fa-table fa-2x mb-3" aria-hidden="true"></i>
  <p>No traceability links yet.</p>
</div>`;

  return `${renderWorkspaceHeader(routeSlug, meta, 'matrix')}
<section class="marreq-section">
  <div class="marreq-section__header">
    <h2 class="marreq-section__title">Traceability matrix</h2>
    <span class="text-muted small">${links?.length ?? 0} links</span>
  </div>
  ${table}
</section>`;
}

/**
 * @param {object} dashboardData - from `GET /api/dashboard`
 * @param {{ namespace: string, projectSlug: string, routeSlug: string, view: string }} parsed
 */
export async function showProjectWorkspace(dashboardData, parsed) {
  const { namespace, projectSlug, routeSlug, view } = parsed;
  const pathname = window.location.pathname;

  try {
    const enc = (x) => encodeURIComponent(x);
    const meta = await apiJson(`/api/project-from-path/${enc(namespace)}/${enc(projectSlug)}`);

    let mainHtml;
    if (view === 'home') {
      mainHtml = renderProjectHome(routeSlug, meta);
    } else if (view === 'requirements') {
      const [requirements, statuses, categories] = await Promise.all([
        apiJson(`/api/projects/${meta.id}/requirements`),
        apiJson('/api/status'),
        apiJson('/api/categories'),
      ]);
      mainHtml = renderRequirementsView(routeSlug, meta, statuses, categories, requirements);
    } else if (view === 'verifications') {
      const [verifications, statuses] = await Promise.all([
        apiJson(`/api/projects/${meta.id}/verifications`),
        apiJson('/api/verification-status'),
      ]);
      const projStatuses = (statuses || []).filter((s) => s.project_id === meta.id);
      mainHtml = renderVerificationsView(routeSlug, meta, projStatuses, verifications);
    } else if (view === 'matrix') {
      const [links, requirements, verifications] = await Promise.all([
        apiJson(`/api/projects/${meta.id}/matrix`),
        apiJson(`/api/projects/${meta.id}/requirements`),
        apiJson(`/api/projects/${meta.id}/verifications`),
      ]);
      const reqById = new Map((requirements || []).map((r) => [r.id, r.reference_code]));
      const verById = new Map(
        (verifications || []).map((v) => [v.id, `${v.reference_code} — ${v.name}`]),
      );
      mainHtml = renderMatrixView(routeSlug, meta, links, reqById, verById);
    } else {
      mainHtml = renderProjectHome(routeSlug, meta);
    }

    const title =
      view === 'home' ? `${meta.name} · Marreq` : `${meta.name} — ${view} · Marreq`;

    await mountAppShell(dashboardData, pathname, mainHtml, title, 'project-workspace');
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    const { showStub } = await import('./spaHome.js');
    await showStub(dashboardData, {
      pathname,
      title: 'Project workspace',
      message: msg,
    });
  }
}
