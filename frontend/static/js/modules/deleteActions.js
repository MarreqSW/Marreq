import { deleteJson } from '../core/net.js';
import { showNotification } from './notifications.js';

export function registerDeleteAction(config) {
  const {
    root = document,
    selector = '[data-action="delete"]',
    getUrl,
    getMessage = () => 'Are you sure? This action cannot be undone.',
    onSuccess = () => window.location.reload(),
    onError = (error) => {
      console.error(error);
      showNotification(error.message || 'Unable to delete item', 'error');
    },
  } = config;

  root.querySelectorAll(selector).forEach((button) => {
    button.addEventListener('click', async (event) => {
      event.preventDefault();

      const url = typeof getUrl === 'function' ? getUrl(button) : button.dataset.deleteUrl;
      if (!url) {
        console.warn('Delete button missing URL', button);
        return;
      }

      const message = getMessage(button);
      if (!window.confirm(message)) {
        return;
      }

      try {
        await deleteJson(url);
        onSuccess(button);
      } catch (error) {
        onError(error, button);
      }
    });
  });
}

