import { showNotification } from './notifications.js';
import { formToJSON } from '../core/net.js';

export function bindModalForm(options) {
  const {
    triggerSelector,
    modalSelector,
    formSelector,
    handleSubmit,
    successMessage = 'Saved successfully',
    errorMessage = 'Unable to complete action',
  } = options;

  const trigger = document.querySelector(triggerSelector);
  const modalElement = document.querySelector(modalSelector);
  const form = document.querySelector(formSelector);

  if (!trigger || !modalElement || !form || !window.bootstrap) {
    return null;
  }

  const modal = new window.bootstrap.Modal(modalElement);

  trigger.addEventListener('click', () => {
    modal.show();
  });

  form.addEventListener('submit', async (event) => {
    event.preventDefault();

    try {
      await handleSubmit({
        form,
        data: formToJSON(form),
        modal,
      });
      showNotification(successMessage, 'success');
      modal.hide();
    } catch (error) {
      console.error(error);
      showNotification(error.message || errorMessage, 'error');
    }
  });

  return { trigger, modal, form };
}

