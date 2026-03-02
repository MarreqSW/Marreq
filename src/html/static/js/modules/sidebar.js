const STORAGE_KEY = 'marreq_sidebar_collapsed';
const DESKTOP_BREAKPOINT = 992;

function isDesktop() {
  return window.innerWidth >= DESKTOP_BREAKPOINT;
}

export function initSidebar() {
  const sidebar = document.getElementById('mainSidebar');
  if (!sidebar) {
    return;
  }

  const sidebarToggle = document.getElementById('sidebarToggle');
  const mobileToggle = document.getElementById('mobileToggle');

  try {
    if (localStorage.getItem(STORAGE_KEY) === 'true' && isDesktop()) {
      sidebar.classList.add('marreq-sidebar--collapsed');
    }
  } catch (error) {
    /* ignore */
  }

  const toggleSidebar = () => {
    const collapsed = sidebar.classList.toggle('marreq-sidebar--collapsed');
    if (isDesktop()) {
      try {
        localStorage.setItem(STORAGE_KEY, String(collapsed));
      } catch (error) {
        /* ignore */
      }
    }
  };

  if (sidebarToggle) {
    sidebarToggle.addEventListener('click', toggleSidebar);
  }

  if (mobileToggle) {
    mobileToggle.addEventListener('click', () => {
      sidebar.classList.toggle('marreq-sidebar--mobile-open');
    });
  }

  document.addEventListener('click', (event) => {
    if (
      window.innerWidth < DESKTOP_BREAKPOINT &&
      !sidebar.contains(event.target) &&
      (!mobileToggle || !mobileToggle.contains(event.target)) &&
      sidebar.classList.contains('marreq-sidebar--mobile-open')
    ) {
      sidebar.classList.remove('marreq-sidebar--mobile-open');
    }
  });
}

