const COOKIE_NAME = 'selected_project_id';
const RESERVED_ROOTS = new Set([
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
const PROJECT_CHILD_SEGMENTS = new Set([
  'admin',
  'baselines',
  'catalog',
  'dashboard',
  'help',
  'matrix',
  'members',
  'reports',
  'requirements',
  'settings',
  'traceability',
  'verifications',
]);

function setCookie(name, value) {
  document.cookie = `${name}=${value}; path=/; max-age=86400`;
}

function getProjectSlugFromPath() {
  const segments = window.location.pathname
    .split('/')
    .filter(Boolean)
    .map((segment) => decodeURIComponent(segment));

  if (segments.length < 1) {
    return null;
  }

  const first = segments[0];
  if (!first || RESERVED_ROOTS.has(first)) {
    return null;
  }

  const second = segments[1];
  if (second && second !== '-' && !PROJECT_CHILD_SEGMENTS.has(second)) {
    return second;
  }

  return first;
}

function resolveProjectId(explicit) {
  if (explicit) {
    return explicit;
  }
  const cookie = document.cookie
    .split(';')
    .map((part) => part.trim())
    .find((part) => part.startsWith(`${COOKIE_NAME}=`));

  if (cookie) {
    return cookie.split('=')[1];
  }

  return null;
}

function projectSlugForId(selector, projectId) {
  const option = Array.from(selector?.options || []).find((item) => item.value === projectId);
  return option?.dataset?.projectSlug || projectId;
}

/** Workspace URLs carry the project slug alone, so drop any legacy group prefix. */
function workspaceSlug(slug) {
  return (slug || '').split('/').filter(Boolean).pop() || null;
}

function resolveProjectIdFromCurrentPath(selector) {
  const currentPathSegment = getProjectSlugFromPath();
  if (!currentPathSegment) {
    return null;
  }

  const options = Array.from(selector?.options || []);

  const bySlug = options.find(
    (item) => workspaceSlug(item.dataset?.projectSlug) === currentPathSegment,
  );
  if (bySlug?.value) {
    return bySlug.value;
  }

  return null;
}

function navigateToProject(projectId, selector) {
  if (!projectId) return;
  const projectSlug = projectSlugForId(selector, projectId);
  if (!projectSlug) return;

  const path = window.location.pathname;
  const segments = path.split('/').filter(Boolean);
  const nextSlug = workspaceSlug(projectSlug);
  if (!nextSlug || !getProjectSlugFromPath()) {
    window.location.reload();
    return;
  }

  const rest =
    segments.length >= 2 && !PROJECT_CHILD_SEGMENTS.has(segments[1])
      ? segments.slice(2)
      : segments.slice(1);
  const newPath = `/${[nextSlug, ...rest].join('/')}`;
  const suffix = window.location.search + window.location.hash;
  window.location.assign(`${newPath}${suffix}`);
}

export function initProjectSelector() {
  const selector = document.getElementById('project-selector');
  if (!selector) {
    return;
  }

  selector.addEventListener('change', () => {
    const projectId = selector.value;
    if (!projectId) {
      return;
    }
    setCookie(COOKIE_NAME, projectId);
    navigateToProject(projectId, selector);
  });

  const hasCookie = document.cookie
    .split(';')
    .map((cookie) => cookie.trim())
    .some((cookie) => cookie.startsWith(`${COOKIE_NAME}=`));

  if (!hasCookie) {
    const firstOption = Array.from(selector.options).find((o) => o.value);
    if (firstOption) {
      selector.value = firstOption.value;
      setCookie(COOKIE_NAME, firstOption.value);
      navigateToProject(firstOption.value, selector);
    }
  } else {
    const activeProject = resolveProjectIdFromCurrentPath(selector) || resolveProjectId();
    if (activeProject) {
      selector.value = activeProject;
    }
  }
}
