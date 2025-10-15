export function init() {
  document.querySelectorAll('[data-action="refresh-stats"]').forEach((button) => {
    button.addEventListener('click', (event) => {
      event.preventDefault();
      window.location.reload();
    });
  });

  setTimeout(() => {
    window.location.reload();
  }, 30000);
}

