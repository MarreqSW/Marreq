const COOKIE_NAME = 'selected_project_id';

function setCookie(name, value) {
  document.cookie = `${name}=${value}; path=/; max-age=86400`;
}

function getProjectSlugFromPath() {
  const match = window.location.pathname.match(/^\/p\/([^/]+)/);
  return match ? decodeURIComponent(match[1]) : null;
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
  const currentSlug = getProjectSlugFromPath();
  if (!currentSlug) {
    return null;
  }
  const option = Array.from(selector?.options || []).find(
    (item) => item.dataset?.projectSlug === currentSlug
  );
  return option?.value || null;
}

function navigateToProject(projectId, selector) {
  if (!projectId) return;
  const projectSlug = projectSlugForId(selector, projectId);
  if (!projectSlug) return;

  const path = window.location.pathname;
  const segments = path.split('/').filter(Boolean);

  if (segments[0] === 'p' && segments.length >= 2) {
    segments[1] = projectSlug;
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
