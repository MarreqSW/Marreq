const COOKIE_NAME = 'selected_project_id';

function setCookie(name, value) {
  document.cookie = `${name}=${value}; path=/; max-age=86400`;
}

function resolveProjectId(explicit) {
  if (explicit) {
    return explicit;
  }

  const match = window.location.pathname.match(/^\/p\/(\d+)/);
  if (match) {
    return match[1];
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

function navigateToProject(projectId) {
  if (!projectId) return;

  const path = window.location.pathname;
  const segments = path.split('/').filter(Boolean);

  if (segments[0] === 'p' && segments.length >= 2) {
    segments[1] = projectId;
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
    navigateToProject(projectId);
  });

  const hasCookie = document.cookie
    .split(';')
    .map((cookie) => cookie.trim())
    .some((cookie) => cookie.startsWith(`${COOKIE_NAME}=`));

  if (!hasCookie) {
    const firstOption = selector.querySelector('option[value]');
    if (firstOption) {
      selector.value = firstOption.value;
      setCookie(COOKIE_NAME, firstOption.value);
      navigateToProject(firstOption.value);
    }
  } else {
    const activeProject = resolveProjectId();
    if (activeProject) {
      selector.value = activeProject;
    }
  }
}

