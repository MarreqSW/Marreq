export function showNotification(message, type = 'info', options = {}) {
  const container = document.createElement('div');
  const alertClass = type === 'success' ? 'alert-success' : type === 'error' ? 'alert-danger' : 'alert-info';
  container.className = `alert ${alertClass} alert-dismissible fade show position-fixed`;
  container.style.top = options.top || '20px';
  container.style.right = options.right || '20px';
  container.style.zIndex = options.zIndex || '9999';
  container.innerHTML = `
    ${message}
    <button type="button" class="btn-close" data-bs-dismiss="alert" aria-label="Close"></button>
  `;

  document.body.appendChild(container);
  const lifespan = options.duration ?? 3000;

  if (lifespan > 0) {
    setTimeout(() => {
      container.classList.remove('show');
      container.addEventListener(
        'transitionend',
        () => {
          container.remove();
        },
        { once: true },
      );
    }, lifespan);
  }

  return container;
}

