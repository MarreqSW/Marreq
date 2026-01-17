/**
 * Tests for modules/modals.js
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { bindModalForm } from '@modules/modals.js';

// Mock dependencies
vi.mock('@modules/notifications.js', () => ({
  showNotification: vi.fn(),
}));

vi.mock('@core/net.js', () => ({
  formToJSON: vi.fn((form) => ({ field: 'value' })),
}));

import { showNotification } from '@modules/notifications.js';
import { formToJSON } from '@core/net.js';

describe('Modals', () => {
  beforeEach(() => {
    document.body.innerHTML = '';
    window.bootstrap = {
      Modal: vi.fn(function(element) {
        this.element = element;
        this.show = vi.fn();
        this.hide = vi.fn();
        return this;
      }),
    };
    vi.clearAllMocks();
  });

  it('should return null if trigger not found', () => {
    document.body.innerHTML = `
      <div id="modal"></div>
      <form id="form"></form>
    `;

    const result = bindModalForm({
      triggerSelector: '#nonexistent',
      modalSelector: '#modal',
      formSelector: '#form',
      handleSubmit: vi.fn(),
    });

    expect(result).toBeNull();
  });

  it('should return null if modal not found', () => {
    document.body.innerHTML = `
      <button id="trigger">Open</button>
      <form id="form"></form>
    `;

    const result = bindModalForm({
      triggerSelector: '#trigger',
      modalSelector: '#nonexistent',
      formSelector: '#form',
      handleSubmit: vi.fn(),
    });

    expect(result).toBeNull();
  });

  it('should return null if form not found', () => {
    document.body.innerHTML = `
      <button id="trigger">Open</button>
      <div id="modal"></div>
    `;

    const result = bindModalForm({
      triggerSelector: '#trigger',
      modalSelector: '#modal',
      formSelector: '#nonexistent',
      handleSubmit: vi.fn(),
    });

    expect(result).toBeNull();
  });

  it('should return null if bootstrap not available', () => {
    window.bootstrap = undefined;
    document.body.innerHTML = `
      <button id="trigger">Open</button>
      <div id="modal"></div>
      <form id="form"></form>
    `;

    const result = bindModalForm({
      triggerSelector: '#trigger',
      modalSelector: '#modal',
      formSelector: '#form',
      handleSubmit: vi.fn(),
    });

    expect(result).toBeNull();
  });

  it('should initialize modal and return object', () => {
    document.body.innerHTML = `
      <button id="trigger">Open</button>
      <div id="modal"></div>
      <form id="form"></form>
    `;

    const result = bindModalForm({
      triggerSelector: '#trigger',
      modalSelector: '#modal',
      formSelector: '#form',
      handleSubmit: vi.fn(),
    });

    expect(result).toBeTruthy();
    expect(result.trigger).toBeTruthy();
    expect(result.modal).toBeTruthy();
    expect(result.form).toBeTruthy();
    expect(window.bootstrap.Modal).toHaveBeenCalled();
  });

  it('should show modal when trigger is clicked', () => {
    document.body.innerHTML = `
      <button id="trigger">Open</button>
      <div id="modal"></div>
      <form id="form"></form>
    `;

    bindModalForm({
      triggerSelector: '#trigger',
      modalSelector: '#modal',
      formSelector: '#form',
      handleSubmit: vi.fn(),
    });

    const modalInstance = window.bootstrap.Modal.mock.results[0].value;
    const trigger = document.getElementById('trigger');
    trigger.click();

    expect(modalInstance.show).toHaveBeenCalled();
  });

  it('should call handleSubmit on form submit', async () => {
    document.body.innerHTML = `
      <button id="trigger">Open</button>
      <div id="modal"></div>
      <form id="form">
        <input name="field" value="value">
        <button type="submit">Submit</button>
      </form>
    `;

    const handleSubmit = vi.fn().mockResolvedValue({});
    bindModalForm({
      triggerSelector: '#trigger',
      modalSelector: '#modal',
      formSelector: '#form',
      handleSubmit,
    });

    const form = document.getElementById('form');
    const submitEvent = new Event('submit', { bubbles: true, cancelable: true });
    form.dispatchEvent(submitEvent);

    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(handleSubmit).toHaveBeenCalledWith({
      form,
      data: { field: 'value' },
      modal: expect.any(Object),
    });
  });

  it('should prevent default form submission', async () => {
    document.body.innerHTML = `
      <button id="trigger">Open</button>
      <div id="modal"></div>
      <form id="form">
        <button type="submit">Submit</button>
      </form>
    `;

    bindModalForm({
      triggerSelector: '#trigger',
      modalSelector: '#modal',
      formSelector: '#form',
      handleSubmit: vi.fn().mockResolvedValue({}),
    });

    const form = document.getElementById('form');
    const submitEvent = new Event('submit', { bubbles: true, cancelable: true });
    const preventDefaultSpy = vi.spyOn(submitEvent, 'preventDefault');
    form.dispatchEvent(submitEvent);

    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(preventDefaultSpy).toHaveBeenCalled();
  });

  it('should show success notification and hide modal on success', async () => {
    document.body.innerHTML = `
      <button id="trigger">Open</button>
      <div id="modal"></div>
      <form id="form"></form>
    `;

    const modalInstance = window.bootstrap.Modal.mock.results[0].value;
    bindModalForm({
      triggerSelector: '#trigger',
      modalSelector: '#modal',
      formSelector: '#form',
      handleSubmit: vi.fn().mockResolvedValue({}),
      successMessage: 'Custom success',
    });

    const form = document.getElementById('form');
    form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));

    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(showNotification).toHaveBeenCalledWith('Custom success', 'success');
    expect(modalInstance.hide).toHaveBeenCalled();
  });

  it('should show error notification on failure', async () => {
    document.body.innerHTML = `
      <button id="trigger">Open</button>
      <div id="modal"></div>
      <form id="form"></form>
    `;

    const error = new Error('Submission failed');
    bindModalForm({
      triggerSelector: '#trigger',
      modalSelector: '#modal',
      formSelector: '#form',
      handleSubmit: vi.fn().mockRejectedValue(error),
      errorMessage: 'Custom error',
    });

    const form = document.getElementById('form');
    form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));

    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(showNotification).toHaveBeenCalledWith('Submission failed', 'error');
  });

  it('should use default success message', async () => {
    document.body.innerHTML = `
      <button id="trigger">Open</button>
      <div id="modal"></div>
      <form id="form"></form>
    `;

    bindModalForm({
      triggerSelector: '#trigger',
      modalSelector: '#modal',
      formSelector: '#form',
      handleSubmit: vi.fn().mockResolvedValue({}),
    });

    const form = document.getElementById('form');
    form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));

    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(showNotification).toHaveBeenCalledWith('Saved successfully', 'success');
  });

  it('should use default error message when error has no message', async () => {
    document.body.innerHTML = `
      <button id="trigger">Open</button>
      <div id="modal"></div>
      <form id="form"></form>
    `;

    const error = new Error();
    error.message = '';
    bindModalForm({
      triggerSelector: '#trigger',
      modalSelector: '#modal',
      formSelector: '#form',
      handleSubmit: vi.fn().mockRejectedValue(error),
    });

    const form = document.getElementById('form');
    form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));

    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(showNotification).toHaveBeenCalledWith('Unable to complete action', 'error');
  });

  it('should log error to console', async () => {
    document.body.innerHTML = `
      <button id="trigger">Open</button>
      <div id="modal"></div>
      <form id="form"></form>
    `;

    const error = new Error('Test error');
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    bindModalForm({
      triggerSelector: '#trigger',
      modalSelector: '#modal',
      formSelector: '#form',
      handleSubmit: vi.fn().mockRejectedValue(error),
    });

    const form = document.getElementById('form');
    form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));

    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(consoleErrorSpy).toHaveBeenCalledWith(error);

    consoleErrorSpy.mockRestore();
  });
});
