const COOKIE_NAME = 'selected_project_id';
const RESERVED_ROOTS = new Set([
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

function setCookie(name, value) {
  document.cookie = `${name}=${value}; path=/; max-age=86400`;
}

function getProjectSlugFromPath() {
  const segments = window.location.pathname
    .split('/')
    .filter(Boolean)
    .map((segment) => decodeURIComponent(segment));

  if (segments.length < 2) {
    return null;
  }

  const [namespace, projectSlug] = segments;
  if (!namespace || !projectSlug || projectSlug === '-' || RESERVED_ROOTS.has(namespace)) {
    return null;
  }

  return `${namespace}/${projectSlug}`;
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

function resolveProjectIdFromCurrentPath(selector) {
  const currentPathSegment = getProjectSlugFromPath();
  if (!currentPathSegment) {
    return null;
  }

  const options = Array.from(selector?.options || []);

  const bySlug = options.find((item) => item.dataset?.projectSlug === currentPathSegment);
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
  const projectSegments = projectSlug.split('/').filter(Boolean);

  if (segments.length >= 2 && projectSegments.length === 2 && getProjectSlugFromPath()) {
    segments[0] = projectSegments[0];
    segments[1] = projectSegments[1];
    const newPath = `/${segments.join('/')}`;
    const suffix = window.location.search + window.location.hash;
    window.location.assign(`${newPath}${suffix}`);
  } else {
    window.location.reload();
  }
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
