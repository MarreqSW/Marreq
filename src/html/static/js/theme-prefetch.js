(function applyStoredTheme() {
  var theme = null;
  try {
    theme = localStorage.getItem('marreq-theme');
  } catch (error) {
    theme = null;
  }

  if (theme !== 'dark' && theme !== 'light') {
    if (window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches) {
      theme = 'dark';
    } else {
      theme = null;
    }
  }

  if (theme === 'dark') {
    document.documentElement.setAttribute('data-theme', 'dark');
  } else if (theme === 'light') {
    document.documentElement.removeAttribute('data-theme');
  }
})();

