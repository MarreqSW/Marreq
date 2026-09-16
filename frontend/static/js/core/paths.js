/**
 * Normalise pathname for SPA routing (trailing slash, index.html).
 * @param {string} pathname
 * @returns {string}
 */
export function normalizeSpaPathname(pathname) {
  if (pathname == null || pathname === '') {
    return '/';
  }
  if (pathname === '/index.html') {
    return '/';
  }
  let p = pathname;
  if (p.length > 1 && p.endsWith('/')) {
    p = p.replace(/\/+$/, '');
    if (p === '') {
      return '/';
    }
  }
  return p;
}

/** First URL segment cannot be a project slug (matches backend `RESERVED_NAMESPACE_SEGMENTS`). */
export const SPA_RESERVED_FIRST_SEGMENTS = new Set([
  'admin',
  'api',
  'cache',
  'change_password',
  'change-password',
  'cleanup_logs',
  'error',
  'export_logs',
  'forgot-password',
  'groups',
  'log_analytics',
  'login',
  'logout',
  'logs',
  'new_project',
  'profile',
  'projects',
  'register',
  'reset-password',
  'static',
  'status',
  'user',
  'verify-email',
]);

const WORKSPACE_TABS = new Set(['requirements', 'verifications', 'matrix']);

/**
 * True for `/{slug}/…` project workspace paths (not reserved top-level routes).
 * @param {string} pathname
 */
export function isProjectWorkspacePath(pathname) {
  const p = normalizeSpaPathname(pathname);
  const parts = p.split('/').filter(Boolean);
  if (parts.length < 1) {
    return false;
  }
  return !SPA_RESERVED_FIRST_SEGMENTS.has(parts[0].toLowerCase());
}

/**
 * Parse project workspace URLs: `/{slug}`, `/{slug}/requirements`, etc.
 * Legacy `/{namespace}/{slug}` and `/{namespace}/{slug}/{tab}` are still accepted.
 * @returns {{ namespace: string, projectSlug: string, routeSlug: string, view: string } | null}
 */
export function parseProjectWorkspaceUrl(pathname) {
  const p = normalizeSpaPathname(pathname);
  const parts = p.split('/').filter(Boolean);
  if (parts.length < 1 || SPA_RESERVED_FIRST_SEGMENTS.has(parts[0].toLowerCase())) {
    return null;
  }

  if (parts.length >= 2 && !WORKSPACE_TABS.has(parts[1])) {
    const namespace = parts[0];
    const projectSlug = parts[1];
    const routeSlug = projectSlug;
    if (parts.length === 2) {
      return { namespace, projectSlug, routeSlug, view: 'home' };
    }
    if (parts.length === 3 && WORKSPACE_TABS.has(parts[2])) {
      return { namespace, projectSlug, routeSlug, view: parts[2] };
    }
    return null;
  }

  const projectSlug = parts[0];
  const routeSlug = projectSlug;
  if (parts.length === 1) {
    return { namespace: '', projectSlug, routeSlug, view: 'home' };
  }
  if (parts.length === 2 && WORKSPACE_TABS.has(parts[1])) {
    return { namespace: '', projectSlug, routeSlug, view: parts[1] };
  }
  return null;
}
