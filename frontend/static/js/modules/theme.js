const STORAGE_KEY = 'marreq-theme';
const root = document.documentElement;

export function applyTheme(theme) {
  if (theme === 'dark') {
    root.setAttribute('data-theme', 'dark');
  } else {
    root.removeAttribute('data-theme');
  }
}

export function getStoredTheme() {
  try {
    return localStorage.getItem(STORAGE_KEY);
  } catch (error) {
    return null;
  }
}

export function setStoredTheme(theme) {
  try {
    localStorage.setItem(STORAGE_KEY, theme);
  } catch (error) {
    /* ignore quota/permission errors */
  }
}

export function resolvePreferredTheme() {
  const stored = getStoredTheme();
  if (stored === 'dark' || stored === 'light') {
    return stored;
  }

  if (window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches) {
    return 'dark';
  }

  return 'light';
}

export function updateToggleMeta(theme) {
  const isDark = theme === 'dark';
  const label = isDark ? 'Switch to light mode' : 'Switch to dark mode';
  document.querySelectorAll('[data-theme-toggle]').forEach((toggle) => {
    toggle.setAttribute('aria-pressed', String(isDark));
    toggle.setAttribute('aria-label', label);
    toggle.setAttribute('title', label);
  });
}

function onThemeToggleClick() {
  const current = root.getAttribute('data-theme') === 'dark' ? 'dark' : 'light';
  const next = current === 'dark' ? 'light' : 'dark';
  applyTheme(next);
  setStoredTheme(next);
  updateToggleMeta(next);
}

/** Call after SPA replaces DOM so new `[data-theme-toggle]` buttons work (avoids duplicating matchMedia listeners). */
export function bindThemeToggles() {
  document.querySelectorAll('[data-theme-toggle]').forEach((toggle) => {
    toggle.addEventListener('click', onThemeToggleClick);
  });
}

export function initThemeControls() {
  const startingTheme = root.getAttribute('data-theme') || resolvePreferredTheme();
  applyTheme(startingTheme);
  updateToggleMeta(startingTheme);

  bindThemeToggles();

  if (window.matchMedia) {
    const query = window.matchMedia('(prefers-color-scheme: dark)');
    const handler = (event) => {
      if (getStoredTheme()) {
        return;
      }
      const nextTheme = event.matches ? 'dark' : 'light';
      applyTheme(nextTheme);
      updateToggleMeta(nextTheme);
    };

    if (query.addEventListener) {
      query.addEventListener('change', handler);
    } else if (query.addListener) {
      query.addListener(handler);
    }
  }
}

