/**
 * SPA login: JSON API + CSRF (Docker/nginx on :8080, /api proxied to Rocket).
 */
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
  const res = await fetch('/api/auth/csrf', { credentials: 'same-origin' });
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

/** SPA dashboard shell is only for `/` (nginx serves SSR HTML for /p/, /projects, … when proxied). */
function shouldBootstrapSpaDashboard() {
  const path = window.location.pathname;
  return path === '/' || path === '/index.html';
}

async function runLoginSpa() {
  const root = document.getElementById('marreq-spa-root');
  if (!root) {
    return;
  }

  if (shouldBootstrapSpaDashboard()) {
    const dashRes = await fetch('/api/dashboard', { credentials: 'same-origin' });
    if (dashRes.ok) {
      const data = await dashRes.json();
      if (data.csrf_token) {
        setCsrfMeta(data.csrf_token);
      }
      const { show: showHome } = await import('./spaHome.js');
      await showHome(data);
      return;
    }
  }

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
          credentials: 'same-origin',
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

/** Sync entry for `app.js` initPageController (async work inside). */
export function init() {
  runLoginSpa().catch((error) => {
    console.error('loginSpa failed:', error);
  });
}
