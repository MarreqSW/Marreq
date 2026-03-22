/**
 * SPA login: JSON API + CSRF (Docker/nginx on :8080, /api proxied to Rocket).
 */
import {
  isProjectWorkspacePath,
  normalizeSpaPathname,
  parseProjectWorkspaceUrl,
} from '../core/paths.js';
import { postJson } from '../core/net.js';

function parseJsonSafe(text) {
  const trimmed = text.replace(/^\uFEFF/, '').trim();
  if (!trimmed) {
    return null;
  }
  try {
    return JSON.parse(trimmed);
  } catch {
    return null;
  }
}

async function fetchCsrfToken() {
  const res = await fetch('/api/auth/csrf', { credentials: 'include' });
  const text = await res.text();
  const data = parseJsonSafe(text);
  if (!res.ok || !data?.csrf_token) {
    const hint = data ? JSON.stringify(data) : text.trim().slice(0, 200);
    throw new Error(`CSRF request failed (${res.status}): ${hint}`);
  }
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

function isSpaEntryPath(pathname) {
  return pathname === '/' || pathname === '/index.html';
}

async function showAuthenticatedShell(data, path) {
  const p = normalizeSpaPathname(path);

  const spaHome = await import('./spaHome.js');

  if (p === '/projects') {
    await spaHome.showProjects(data);
    return;
  }
  if (p === '/') {
    await spaHome.show(data);
    return;
  }

  if (
    p.startsWith('/admin') ||
    p === '/logs' ||
    p === '/new_project' ||
    p.startsWith('/user/') ||
    p === '/change_password'
  ) {
    await spaHome.showStub(data, {
      pathname: p,
      title: 'Not available in SPA',
      message:
        'This screen is not implemented in the current SPA shell. Use the JSON API (see doc/API.md) or return to the dashboard.',
    });
    return;
  }

  if (isProjectWorkspacePath(p)) {
    const parsed = parseProjectWorkspaceUrl(p);
    if (parsed) {
      const { showProjectWorkspace } = await import('./spaProjectWorkspace.js');
      await showProjectWorkspace(data, parsed);
      return;
    }
    await spaHome.showStub(data, {
      pathname: p,
      title: 'Project workspace',
      message:
        'This project URL is not available in the SPA shell yet (e.g. requirement detail or nested paths). Use the JSON API or return to the dashboard.',
    });
    return;
  }

  await spaHome.showStub(data, {
    pathname: p,
    title: 'Unknown page',
    message: 'No client view is registered for this path.',
  });
}

const fetchSessionOpts = {
  credentials: 'include',
  cache: 'no-store',
  headers: { Accept: 'application/json' },
};

async function fetchDashboardPayload() {
  const dashRes = await fetch('/api/dashboard', fetchSessionOpts);
  const text = await dashRes.text();
  const data = parseJsonSafe(text);
  if (!dashRes.ok) {
    return {
      ok: false,
      status: dashRes.status,
      hint: data ? JSON.stringify(data).slice(0, 300) : text.trim().slice(0, 200),
    };
  }
  if (!data || typeof data !== 'object' || !data.user) {
    return { ok: false, status: dashRes.status, hint: 'dashboard JSON missing user' };
  }
  return { ok: true, data };
}

/** Minimal HTML escape for user-visible error snippets (not for full documents). */
function escapeHtml(s) {
  return String(s)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');
}

function escapeAttr(s) {
  return escapeHtml(String(s)).replace(/"/g, '&quot;');
}

function renderSessionProbeError(root, status, hint) {
  const loc = window.location.pathname + window.location.search + window.location.hash;
  const home = `<a href="/" class="alert-link">Home</a>`;
  const retry = `<a href="${escapeAttr(loc)}" class="alert-link">Retry</a>`;
  root.innerHTML = `<div class="alert alert-danger m-3" role="alert">
    <strong>Could not verify your session</strong> (HTTP ${status}). ${escapeHtml(hint)}
    <p class="mb-0 mt-2">${home} · ${retry}</p>
  </div>`;
}

async function prepareLoginForm(root) {
  const form = root.querySelector('.marreq-login__form');
  if (!form) {
    return;
  }

  try {
    const token = await fetchCsrfToken();
    setCsrfMeta(token);
  } catch (e) {
    console.warn('Could not prefetch CSRF token', e);
  }

  form.addEventListener('submit', async (event) => {
    event.preventDefault();
    const username = form.querySelector('#username')?.value?.trim();
    const password = form.querySelector('#password')?.value ?? '';
    let token = document.querySelector('meta[name="csrf-token"]')?.getAttribute('content');
    if (!token) {
      try {
        token = await fetchCsrfToken();
        setCsrfMeta(token);
      } catch (e) {
        console.error(e);
        return;
      }
    }

    let alertBox = form.querySelector('.js-login-alert');
    if (alertBox) {
      alertBox.remove();
    }

    try {
      await postJson(
        '/api/auth/login',
        { username, password },
        {
          credentials: 'include',
          headers: { 'X-CSRF-Token': token },
        },
      );
      window.location.reload();
    } catch (err) {
      const msg =
        err.payload?.message ||
        err.message ||
        'Login failed. Check username and password.';
      alertBox = document.createElement('div');
      alertBox.className = 'alert alert-danger c-alert js-login-alert';
      alertBox.textContent = msg;
      const submit = form.querySelector('button[type="submit"]');
      form.insertBefore(alertBox, submit);
    }
  });
}

async function runLoginSpa() {
  const root = document.getElementById('marreq-spa-root');
  if (!root) {
    return;
  }

  const path = normalizeSpaPathname(window.location.pathname);

  document.documentElement.classList.add('marreq-bootstrapping');

  // Vite (and Docker nginx) serve this same index shell for every client route. Always probe the
  // session; otherwise deep links skip the check and show the login form even when cookies are valid.
  try {
    const meRes = await fetch('/api/auth/me', fetchSessionOpts);

    if (meRes.status === 401) {
      if (!isSpaEntryPath(path)) {
        window.location.replace('/');
        return;
      }
      await prepareLoginForm(root);
      return;
    }

    if (!meRes.ok) {
      const hint = (await meRes.text()).trim().slice(0, 400);
      renderSessionProbeError(root, meRes.status, hint || meRes.statusText || 'Unexpected response');
      return;
    }

    await meRes.text().catch(() => {});

    const dash = await fetchDashboardPayload();
    if (!dash.ok) {
      console.error('Session is valid but dashboard request failed', dash.status, dash.hint);
      root.innerHTML = `<div class="alert alert-danger m-3" role="alert">
          <strong>Could not load the dashboard</strong> (HTTP ${dash.status}). The API returned an error — check the browser console and that the backend is running.
          If you use <code>npm run preview</code>, ensure <code>vite.config.ts</code> proxies <code>/api</code> to Rocket (same as dev).
        </div>`;
      return;
    }

    await showAuthenticatedShell(dash.data, path);
    return;
  } catch (e) {
    console.warn('Session probe failed', e);
    const msg = e instanceof Error ? e.message : String(e);
    if (!isSpaEntryPath(path)) {
      root.innerHTML = `<div class="alert alert-warning m-3" role="alert">
        <strong>Could not reach the API</strong> while opening <code>${escapeHtml(path)}</code>.
        <p class="mb-1 small text-muted">${escapeHtml(msg)}</p>
        <p class="mb-0"><a href="/" class="alert-link">Go to home</a> · <a href="${escapeAttr(window.location.pathname)}" class="alert-link">Reload this page</a></p>
      </div>`;
      return;
    }
    await prepareLoginForm(root);
  } finally {
    document.documentElement.classList.remove('marreq-bootstrapping');
  }
}

/** Sync entry for `app.js` initPageController (async work inside). */
export function init() {
  runLoginSpa().catch((error) => {
    console.error('loginSpa failed:', error);
  });
}
