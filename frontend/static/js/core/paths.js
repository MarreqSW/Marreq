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

/** First URL segment cannot be a project namespace (matches backend `RESERVED_NAMESPACE_SEGMENTS`). */
export const SPA_RESERVED_FIRST_SEGMENTS = new Set([
  'admin',
  'api',
  'cache',
  'change_password',
  'cleanup_logs',
  'error',
  'export_logs',
  'groups',
  'log_analytics',
  'login',
  'logout',
  'logs',
  'new_project',
  'profile',
  'projects',
  'static',
  'status',
  'user',
]);

/**
 * True for `/{namespace}/{project}/…` style paths (not reserved top-level routes).
 * @param {string} pathname
 */
export function isProjectWorkspacePath(pathname) {
  const p = normalizeSpaPathname(pathname);
  const parts = p.split('/').filter(Boolean);
  if (parts.length < 2) {
    return false;
  }
  return !SPA_RESERVED_FIRST_SEGMENTS.has(parts[0].toLowerCase());
}

/**
 * Parse project workspace URLs: `/{ns}/{slug}`, `/{ns}/{slug}/requirements`, etc.
 * @returns {{ namespace: string, projectSlug: string, routeSlug: string, view: string } | null}
 */
export function parseProjectWorkspaceUrl(pathname) {
  const p = normalizeSpaPathname(pathname);
  const parts = p.split('/').filter(Boolean);
  if (parts.length < 2 || SPA_RESERVED_FIRST_SEGMENTS.has(parts[0].toLowerCase())) {
    return null;
  }
  const namespace = parts[0];
  const projectSlug = parts[1];
  const routeSlug = `${namespace}/${projectSlug}`;
  if (parts.length === 2) {
    return { namespace, projectSlug, routeSlug, view: 'home' };
  }
  if (parts.length === 3) {
    const tab = parts[2];
    if (tab === 'requirements' || tab === 'verifications' || tab === 'matrix') {
      return { namespace, projectSlug, routeSlug, view: tab };
    }
  }
  return null;
}
