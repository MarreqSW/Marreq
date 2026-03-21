/**
 * Tests for modules/deleteActions.js
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { registerDeleteAction } from '@modules/deleteActions.js';

// Mock dependencies
vi.mock('@core/net.js', () => ({
  deleteJson: vi.fn(),
}));

vi.mock('@modules/notifications.js', () => ({
  showNotification: vi.fn(),
}));

import { deleteJson } from '@core/net.js';
import { showNotification } from '@modules/notifications.js';

describe('Delete Actions', () => {
  beforeEach(() => {
    document.body.innerHTML = '';
    vi.clearAllMocks();
    window.confirm = vi.fn(() => true);
  });

  it('should register delete action on elements with data-action="delete"', () => {
    document.body.innerHTML = `
      <button data-action="delete" data-delete-url="/api/delete/1">Delete</button>
    `;

    registerDeleteAction({});

    const button = document.querySelector('[data-action="delete"]');
    expect(button).toBeTruthy();
  });

  it('should call deleteJson when confirmed', async () => {
    document.body.innerHTML = `
      <button data-action="delete" data-delete-url="/api/delete/1">Delete</button>
    `;
    window.confirm = vi.fn(() => true);
    deleteJson.mockResolvedValueOnce({});

    registerDeleteAction({
      onSuccess: vi.fn(),
    });

    const button = document.querySelector('[data-action="delete"]');
    button.click();

    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(window.confirm).toHaveBeenCalled();
    expect(deleteJson).toHaveBeenCalledWith('/api/delete/1');
  });

  it('should not call deleteJson when not confirmed', async () => {
    document.body.innerHTML = `
      <button data-action="delete" data-delete-url="/api/delete/1">Delete</button>
    `;
    window.confirm = vi.fn(() => false);

    registerDeleteAction({});

    const button = document.querySelector('[data-action="delete"]');
    button.click();

    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(window.confirm).toHaveBeenCalled();
    expect(deleteJson).not.toHaveBeenCalled();
  });

  it('should use custom selector', async () => {
    document.body.innerHTML = `
      <button class="custom-delete" data-url="/api/delete/1">Delete</button>
    `;
    window.confirm = vi.fn(() => true);
    deleteJson.mockResolvedValueOnce({});

    registerDeleteAction({
      selector: '.custom-delete',
      getUrl: (button) => button.dataset.url,
    });

    const button = document.querySelector('.custom-delete');
    button.click();

    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(deleteJson).toHaveBeenCalledWith('/api/delete/1');
  });

  it('should use getUrl function when provided', async () => {
    document.body.innerHTML = `
      <button data-action="delete" data-id="123">Delete</button>
    `;
    window.confirm = vi.fn(() => true);
    deleteJson.mockResolvedValueOnce({});

    const getUrl = vi.fn((button) => `/api/delete/${button.dataset.id}`);

    registerDeleteAction({
      getUrl,
    });

    const button = document.querySelector('[data-action="delete"]');
    button.click();

    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(getUrl).toHaveBeenCalledWith(button);
    expect(deleteJson).toHaveBeenCalledWith('/api/delete/123');
  });

  it('should use default message when getMessage not provided', async () => {
    document.body.innerHTML = `
      <button data-action="delete" data-delete-url="/api/delete/1">Delete</button>
    `;
    window.confirm = vi.fn(() => true);
    deleteJson.mockResolvedValueOnce({});

    registerDeleteAction({});

    const button = document.querySelector('[data-action="delete"]');
    button.click();

    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(window.confirm).toHaveBeenCalledWith('Are you sure? This action cannot be undone.');
  });

  it('should use custom message from getMessage', async () => {
    document.body.innerHTML = `
      <button data-action="delete" data-delete-url="/api/delete/1" data-name="Item">Delete</button>
    `;
    window.confirm = vi.fn(() => true);
    deleteJson.mockResolvedValueOnce({});

    registerDeleteAction({
      getMessage: (button) => `Delete ${button.dataset.name}?`,
    });

    const button = document.querySelector('[data-action="delete"]');
    button.click();

    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(window.confirm).toHaveBeenCalledWith('Delete Item?');
  });

  it('should call onSuccess after successful delete', async () => {
    document.body.innerHTML = `
      <button data-action="delete" data-delete-url="/api/delete/1">Delete</button>
    `;
    window.confirm = vi.fn(() => true);
    deleteJson.mockResolvedValueOnce({});
    const onSuccess = vi.fn();

    registerDeleteAction({
      onSuccess,
    });

    const button = document.querySelector('[data-action="delete"]');
    button.click();

    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(onSuccess).toHaveBeenCalledWith(button);
  });

  it('should call onError on delete failure', async () => {
    document.body.innerHTML = `
      <button data-action="delete" data-delete-url="/api/delete/1">Delete</button>
    `;
    window.confirm = vi.fn(() => true);
    const error = new Error('Delete failed');
    deleteJson.mockRejectedValueOnce(error);
    const onError = vi.fn();

    registerDeleteAction({
      onError,
    });

    const button = document.querySelector('[data-action="delete"]');
    button.click();

    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(onError).toHaveBeenCalledWith(error, button);
  });

  it('should use default onError handler', async () => {
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});
    document.body.innerHTML = `
      <button data-action="delete" data-delete-url="/api/delete/1">Delete</button>
    `;
    window.confirm = vi.fn(() => true);
    const error = new Error('Delete failed');
    deleteJson.mockRejectedValueOnce(error);

    registerDeleteAction({});

    const button = document.querySelector('[data-action="delete"]');
    button.click();

    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(showNotification).toHaveBeenCalledWith('Delete failed', 'error');
    consoleError.mockRestore();
  });

  it('should use default onSuccess handler (reload)', async () => {
    document.body.innerHTML = `
      <button data-action="delete" data-delete-url="/api/delete/1">Delete</button>
    `;
    window.confirm = vi.fn(() => true);
    deleteJson.mockResolvedValueOnce({});
    window.location.reload = vi.fn();

    registerDeleteAction({});

    const button = document.querySelector('[data-action="delete"]');
    button.click();

    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(window.location.reload).toHaveBeenCalled();
  });

  it('should prevent default event behavior', async () => {
    document.body.innerHTML = `
      <form>
        <button data-action="delete" data-delete-url="/api/delete/1" type="submit">Delete</button>
      </form>
    `;
    window.confirm = vi.fn(() => true);
    deleteJson.mockResolvedValueOnce({});

    registerDeleteAction({});

    const button = document.querySelector('[data-action="delete"]');
    const event = new MouseEvent('click', { bubbles: true, cancelable: true });
    const preventDefaultSpy = vi.spyOn(event, 'preventDefault');

    button.dispatchEvent(event);

    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(preventDefaultSpy).toHaveBeenCalled();
  });

  it('should warn when URL is missing', async () => {
    document.body.innerHTML = `
      <button data-action="delete">Delete</button>
    `;
    window.confirm = vi.fn(() => true);
    const consoleWarnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});

    registerDeleteAction({});

    const button = document.querySelector('[data-action="delete"]');
    button.click();

    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(consoleWarnSpy).toHaveBeenCalledWith('Delete button missing URL', button);
    expect(deleteJson).not.toHaveBeenCalled();

    consoleWarnSpy.mockRestore();
  });

  it('should work with custom root element', async () => {
    document.body.innerHTML = `
      <div id="container">
        <button data-action="delete" data-delete-url="/api/delete/1">Delete</button>
      </div>
      <button data-action="delete" data-delete-url="/api/delete/2">Delete</button>
    `;
    window.confirm = vi.fn(() => true);
    deleteJson.mockResolvedValue({});

    const container = document.getElementById('container');
    registerDeleteAction({
      root: container,
    });

    const button1 = container.querySelector('[data-action="delete"]');
    const button2 = document.querySelectorAll('[data-action="delete"]')[1];

    button1.click();
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(deleteJson).toHaveBeenCalledWith('/api/delete/1');

    button2.click();
    await new Promise((resolve) => setTimeout(resolve, 0));
    // Should not be called again because button2 is outside root
    expect(deleteJson).toHaveBeenCalledTimes(1);
  });
});
