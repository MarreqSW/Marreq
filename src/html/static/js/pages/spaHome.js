/**
 * Post-login dashboard: parity with templates/index.html.hbs + layout (sidebar, cards, footer, diff modal shell).
 */
import { bindThemeToggles, updateToggleMeta } from '../modules/theme.js';
import { initSidebar } from '../modules/sidebar.js';
import { postJson } from '../core/net.js';

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

async function fetchCsrfToken() {
  const res = await fetch('/api/auth/csrf', { credentials: 'same-origin' });
  if (!res.ok) {
    throw new Error(`CSRF request failed (${res.status})`);
  }
  const data = await res.json();
  return data.csrf_token;
}

function setCsrfMeta(token) {
  let meta = document.querySelector('meta[name="csrf-token"]');
  if (!meta) {
    meta = document.createElement('meta');
    meta.setAttribute('name', 'csrf-token');
    document.head.appendChild(meta);
  }
  meta.setAttribute('content', token);
}

function reqWord(n) {
  const x = Number(n);
  return x === 1 ? '1 requirement' : `${x} requirements`;
}

function testWord(n) {
  const x = Number(n);
  return x === 1 ? '1 test' : `${x} tests`;
}

function renderQuickActionCard({ href, icon, label, modifier, disabled, title }) {
  const mod = modifier ? ` ${escapeAttr(modifier)}` : '';
  const titleAttr = title ? ` title="${escapeAttr(title)}"` : '';
  if (disabled) {
    return `<span class="marreq-action-card${mod} marreq-nav-link--disabled" aria-disabled="true"${titleAttr}>${
      icon ? `<i class="${escapeAttr(icon)}" aria-hidden="true"></i>` : ''
    }<span>${escapeHtml(label)}</span></span>`;
  }
  return `<a href="${escapeAttr(href)}" class="marreq-action-card${mod}"${titleAttr}>${
    icon ? `<i class="${escapeAttr(icon)}" aria-hidden="true"></i>` : ''
  }<span>${escapeHtml(label)}</span></a>`;
}

function renderSidebar(user) {
  const admin = user.is_admin;
  return `<aside class="marreq-sidebar" id="mainSidebar">
  <div class="marreq-sidebar__header">
    <a href="/" class="marreq-sidebar__brand">
      <span class="marreq-sidebar__brand-text">Marreq</span>
    </a>
    <div class="marreq-sidebar__header-actions">
      <button data-theme-toggle class="theme-toggle c-theme-toggle" type="button" aria-label="Toggle dark mode" aria-pressed="false" title="Toggle dark mode">
        <span class="theme-toggle-icon theme-toggle-icon-sun c-theme-toggle__icon c-theme-toggle__icon--sun" aria-hidden="true">☀</span>
        <span class="theme-toggle-icon theme-toggle-icon-moon c-theme-toggle__icon c-theme-toggle__icon--moon" aria-hidden="true">🌙</span>
      </button>
      <button type="button" class="marreq-sidebar__toggle" id="sidebarToggle" aria-label="Toggle sidebar">
        <i class="fas fa-chevron-left" aria-hidden="true"></i>
      </button>
    </div>
  </div>
  <nav class="marreq-sidebar__nav" aria-label="Main navigation">
    <div class="marreq-nav-section">
      <h3 class="marreq-nav-section__title">Overview</h3>
      <ul class="marreq-nav-list">
        <li>
          <a href="/" class="marreq-nav-link marreq-nav-link--active" aria-current="page">
            <i class="fas fa-chart-line" aria-hidden="true"></i>
            <span class="marreq-nav-link__label">Dashboard</span>
          </a>
        </li>
        <li>
          <a href="#activity" class="marreq-nav-link marreq-nav-link--disabled" title="Coming soon">
            <i class="fas fa-stream" aria-hidden="true"></i>
            <span class="marreq-nav-link__label">Activity Feed</span>
            <span class="marreq-badge marreq-badge--soon">Soon</span>
          </a>
        </li>
      </ul>
    </div>
    <div class="marreq-nav-section">
      <h3 class="marreq-nav-section__title">Projects</h3>
      <ul class="marreq-nav-list">
        <li>
          <a href="/projects" class="marreq-nav-link">
            <i class="fas fa-folder-tree" aria-hidden="true"></i>
            <span class="marreq-nav-link__label">All Projects</span>
          </a>
        </li>
        ${
          admin
            ? `<li>
          <a href="/new_project" class="marreq-nav-link">
            <i class="fas fa-plus-circle" aria-hidden="true"></i>
            <span class="marreq-nav-link__label">Create Project</span>
          </a>
        </li>`
            : ''
        }
      </ul>
    </div>
    <div class="marreq-nav-section">
      <h3 class="marreq-nav-section__title">Workflows</h3>
      <ul class="marreq-nav-list">
        <li>
          <a href="#change-requests" class="marreq-nav-link marreq-nav-link--disabled" title="Coming soon">
            <i class="fas fa-code-branch" aria-hidden="true"></i>
            <span class="marreq-nav-link__label">Change Requests</span>
            <span class="marreq-badge marreq-badge--soon">Soon</span>
          </a>
        </li>
        <li>
          <a href="#reviews" class="marreq-nav-link marreq-nav-link--disabled" title="Coming soon">
            <i class="fas fa-user-check" aria-hidden="true"></i>
            <span class="marreq-nav-link__label">Reviews</span>
            <span class="marreq-badge marreq-badge--soon">Soon</span>
          </a>
        </li>
      </ul>
    </div>
    ${
      admin
        ? `<div class="marreq-nav-section">
      <h3 class="marreq-nav-section__title">Administration</h3>
      <ul class="marreq-nav-list">
        <li>
          <a href="/admin" class="marreq-nav-link">
            <i class="fas fa-shield-alt" aria-hidden="true"></i>
            <span class="marreq-nav-link__label">Admin Panel</span>
          </a>
        </li>
        <li>
          <a href="/admin/users" class="marreq-nav-link">
            <i class="fas fa-users-cog" aria-hidden="true"></i>
            <span class="marreq-nav-link__label">User Management</span>
          </a>
        </li>
        <li>
          <a href="/logs" class="marreq-nav-link">
            <i class="fas fa-list-alt" aria-hidden="true"></i>
            <span class="marreq-nav-link__label">System Logs</span>
          </a>
        </li>
      </ul>
    </div>`
        : ''
    }
  </nav>
  <div class="marreq-sidebar__footer">
    <div class="marreq-user-info">
      <div class="marreq-user-info__avatar">
        <i class="fas fa-user-circle" aria-hidden="true"></i>
      </div>
      <div class="marreq-user-info__details">
        <div class="marreq-user-info__name">${escapeHtml(user.name || user.username)}</div>
        <div class="marreq-user-info__email">${escapeHtml(user.email)}</div>
      </div>
      <div class="marreq-user-info__actions">
        <div class="dropdown">
          <button class="marreq-user-menu-toggle btn btn-link p-0" type="button" data-bs-toggle="dropdown" aria-expanded="false" title="User menu">
            <i class="fas fa-cog marreq-user-menu-toggle__icon" aria-hidden="true"></i>
            <span class="visually-hidden">User menu</span>
          </button>
          <ul class="dropdown-menu dropdown-menu-end">
            <li>
              <a class="dropdown-item" href="/user/profile">
                <i class="fas fa-user me-2"></i> My Profile
              </a>
            </li>
            <li>
              <a class="dropdown-item" href="/change_password">
                <i class="fas fa-key me-2"></i> Change Password
              </a>
            </li>
            <li><hr class="dropdown-divider"></li>
            <li>
              <button type="button" class="dropdown-item text-danger" data-action="spa-logout" style="background:none;border:none;width:100%;text-align:left">
                <i class="fas fa-sign-out-alt me-2"></i> Logout
              </button>
            </li>
          </ul>
        </div>
      </div>
    </div>
  </div>
</aside>`;
}

function renderHeader(user) {
  const title = user.name
    ? `Welcome back, ${escapeHtml(user.name)}`
    : 'Requirements Dashboard';
  return `<header class="marreq-main__header">
  <button type="button" class="marreq-mobile-toggle" id="mobileToggle" aria-label="Toggle navigation">
    <i class="fas fa-bars" aria-hidden="true"></i>
  </button>
  <div class="marreq-main__title">
    <h1>${title}</h1>
    <p class="marreq-main__subtitle">Manage requirements, tests, and traceability across all your projects</p>
  </div>
</header>`;
}

function renderProjectCard(p) {
  const slug = escapeAttr(p.project_slug);
  const name = escapeHtml(p.name);
  const badge = escapeAttr(p.project_status_badge || 'secondary');
  const statusLabel = escapeHtml(p.status_id || '');
  const initial = escapeHtml(p.project_initial || '#');
  const ownerLine =
    p.project_owner_name != null && p.project_owner_name !== ''
      ? `<p class="marreq-project-card__owner">Owned by ${escapeHtml(p.project_owner_name)}</p>`
      : '';
  const desc =
    p.description != null && String(p.description).trim() !== ''
      ? `<p class="marreq-project-card__description">${escapeHtml(p.description)}</p>`
      : '';
  const roleText = p.role_label ? `Role: ${escapeHtml(p.role_label)}` : 'Role: Member';
  let dateMeta = '';
  if (p.update_date) {
    dateMeta = `<div class="marreq-project-card__meta-item">
        <i class="fas fa-clock-rotate-left" aria-hidden="true"></i>
        <span>Updated ${escapeHtml(p.update_date)}</span>
      </div>`;
  } else if (p.creation_date) {
    dateMeta = `<div class="marreq-project-card__meta-item">
        <i class="fas fa-calendar-plus" aria-hidden="true"></i>
        <span>Created ${escapeHtml(p.creation_date)}</span>
      </div>`;
  }
  return `<article class="marreq-project-card marreq-project-card--${badge}">
  <div class="marreq-project-card__status-bar" aria-hidden="true"></div>
  <div class="marreq-project-card__content">
    <div class="marreq-project-card__header">
      <div class="marreq-project-card__title-group">
        <span class="marreq-project-card__icon" aria-hidden="true">${initial}</span>
        <div class="marreq-project-card__titles">
          <h3>${name}</h3>
          ${ownerLine}
        </div>
      </div>
      <span class="marreq-project-card__status marreq-badge marreq-badge--${badge}">${statusLabel}</span>
    </div>
    ${desc}
    <div class="marreq-project-card__meta">
      <div class="marreq-project-card__meta-item">
        <i class="fas fa-clipboard-list" aria-hidden="true"></i>
        <span>${escapeHtml(reqWord(p.requirements_count))}</span>
      </div>
      <div class="marreq-project-card__meta-item">
        <i class="fas fa-vial" aria-hidden="true"></i>
        <span>${escapeHtml(testWord(p.tests_count))}</span>
      </div>
      <div class="marreq-project-card__meta-item">
        <i class="fas fa-id-badge" aria-hidden="true"></i>
        <span>${roleText}</span>
      </div>
      ${dateMeta}
    </div>
    <div class="marreq-project-card__actions">
      <a href="/p/${slug}" class="marreq-btn marreq-btn--primary">
        <i class="fas fa-eye" aria-hidden="true"></i>
        Open
      </a>
      <a href="/p/${slug}/requirements" class="marreq-btn marreq-btn--outline">
        <i class="fas fa-clipboard-list" aria-hidden="true"></i>
        Requirements
      </a>
    </div>
  </div>
</article>`;
}

function renderPortal(user, projects) {
  const quick = [
    renderQuickActionCard({
      href: '/projects',
      icon: 'fas fa-folder-tree',
      label: 'Browse Projects',
      modifier: 'marreq-action-card--primary',
    }),
  ];
  if (user.is_admin) {
    quick.push(
      renderQuickActionCard({
        href: '/new_project',
        icon: 'fas fa-folder-plus',
        label: 'Create Project',
      }),
      renderQuickActionCard({
        icon: 'fas fa-code-branch',
        label: 'Failing Tests',
        disabled: true,
        title: 'Coming soon',
      }),
      renderQuickActionCard({
        href: '/admin',
        icon: 'fas fa-shield-alt',
        label: 'Admin Panel',
      }),
    );
  }

  const grid =
    projects && projects.length > 0
      ? `<div class="marreq-projects-grid">${projects.map((p) => renderProjectCard(p)).join('')}</div>`
      : `<div class="marreq-empty-state">
          <i class="fas fa-folder-open" aria-hidden="true"></i>
          <h3>No Projects Yet</h3>
          <p>Create your first project to start managing requirements and tests</p>
          ${
            user.is_admin
              ? `<a href="/new_project" class="marreq-btn marreq-btn--primary">
            <i class="fas fa-plus" aria-hidden="true"></i>
            Create Project
          </a>`
              : ''
          }
        </div>`;

  return `<div class="marreq-portal">
  ${renderSidebar(user)}
  <main class="marreq-main" id="mainContent">
    ${renderHeader(user)}
    <section class="marreq-section">
      <h2 class="marreq-section__title">Quick Actions</h2>
      <div class="marreq-quick-actions">${quick.join('')}</div>
    </section>
    <section class="marreq-section">
      <div class="marreq-section__header">
        <h2 class="marreq-section__title">Your Projects</h2>
        <a href="/projects" class="marreq-link-arrow">
          View all
          <i class="fas fa-arrow-right" aria-hidden="true"></i>
        </a>
      </div>
      ${grid}
    </section>
    <section class="marreq-section">
      <h2 class="marreq-section__title">Recent Activity</h2>
      <div class="marreq-placeholder-feature">
        <i class="fas fa-clock" aria-hidden="true"></i>
        <h3>Activity Feed Coming Soon</h3>
        <p>Track project updates, requirement changes, and team activities in real-time</p>
      </div>
    </section>
  </main>
</div>`;
}

function renderLayoutBody(user, projects) {
  return `<main class="o-main-content">
${renderPortal(user, projects)}
</main>
<footer class="footer c-footer">
  <p>&copy; 2026 Marreq - Requirements Management System</p>
</footer>
<div class="modal fade" id="requirementDiffModal" tabindex="-1" aria-labelledby="requirementDiffModalLabel" aria-hidden="true">
  <div class="modal-dialog modal-lg modal-dialog-scrollable">
    <div class="modal-content">
      <div class="modal-header">
        <h5 class="modal-title" id="requirementDiffModalLabel">Requirement diff</h5>
        <button type="button" class="btn-close" data-bs-dismiss="modal" aria-label="Close"></button>
      </div>
      <div class="modal-body">
        <div class="mb-3">
          <small class="text-muted">
            <span class="text-danger">Red</span> = Removed · <span class="text-success">Green</span> = Added · <span class="text-muted">Gray</span> = Unchanged
          </small>
        </div>
        <div id="requirementDiffContent"></div>
      </div>
    </div>
  </div>
</div>`;
}

/**
 * @param {object} data - JSON from `GET /api/dashboard`
 */
export async function show(data) {
  const { user, projects, csrf_token: csrfToken } = data;
  if (csrfToken) {
    setCsrfMeta(csrfToken);
  }

  document.title = 'Dashboard - Marreq';
  document.body.classList.remove('login-page');
  document.body.dataset.page = 'index';

  document.body.innerHTML = renderLayoutBody(user, projects || []);

  const theme = document.documentElement.getAttribute('data-theme') === 'dark' ? 'dark' : 'light';
  updateToggleMeta(theme);
  bindThemeToggles();
  initSidebar();

  document.body.querySelector('[data-action="spa-logout"]')?.addEventListener('click', async () => {
    try {
      const token = await fetchCsrfToken();
      await postJson(
        '/api/auth/logout',
        {},
        {
          credentials: 'same-origin',
          headers: { 'X-CSRF-Token': token },
        },
      );
    } catch (e) {
      console.warn('Logout request failed', e);
    }
    window.location.reload();
  });
}
