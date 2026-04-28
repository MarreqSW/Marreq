/**
 * Tests for modules/notifications.js
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { showNotification } from '@modules/notifications.js';

describe('Notifications', () => {
  beforeEach(() => {
    document.body.innerHTML = '';
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('should create and append notification element', () => {
    const notification = showNotification('Test message');

    expect(notification).toBeTruthy();
    expect(document.body.contains(notification)).toBe(true);
    expect(notification.textContent).toContain('Test message');
  });

  it('should use info type by default', () => {
    const notification = showNotification('Test message');

    expect(notification.className).toContain('alert-info');
    expect(notification.className).not.toContain('alert-success');
    expect(notification.className).not.toContain('alert-danger');
  });

  it('should use success type when specified', () => {
    const notification = showNotification('Success message', 'success');

    expect(notification.className).toContain('alert-success');
    expect(notification.className).not.toContain('alert-info');
    expect(notification.className).not.toContain('alert-danger');
  });

  it('should use error type when specified', () => {
    const notification = showNotification('Error message', 'error');

    expect(notification.className).toContain('alert-danger');
    expect(notification.className).not.toContain('alert-info');
    expect(notification.className).not.toContain('alert-success');
  });

  it('should include close button', () => {
    const notification = showNotification('Test message');

    const closeButton = notification.querySelector('.btn-close');
    expect(closeButton).toBeTruthy();
    expect(closeButton.getAttribute('aria-label')).toBe('Close');
  });

  it('should use default positioning', () => {
    const notification = showNotification('Test message');

    expect(notification.style.top).toBe('20px');
    expect(notification.style.right).toBe('20px');
    expect(notification.style.zIndex).toBe('9999');
  });

  it('should use custom positioning options', () => {
    const notification = showNotification('Test message', 'info', {
      top: '10px',
      right: '30px',
      zIndex: '10000',
    });

    expect(notification.style.top).toBe('10px');
    expect(notification.style.right).toBe('30px');
    expect(notification.style.zIndex).toBe('10000');
  });

  it('should auto-dismiss after default duration', () => {
    const notification = showNotification('Test message');

    expect(notification.classList.contains('show')).toBe(true);
    expect(document.body.contains(notification)).toBe(true);

    vi.advanceTimersByTime(3000);

    expect(notification.classList.contains('show')).toBe(false);
  });

  it('should auto-dismiss after custom duration', () => {
    const notification = showNotification('Test message', 'info', { duration: 5000 });

    vi.advanceTimersByTime(5000);

    expect(notification.classList.contains('show')).toBe(false);
  });

  it('should not auto-dismiss when duration is 0', () => {
    const notification = showNotification('Test message', 'info', { duration: 0 });

    vi.advanceTimersByTime(10000);

    expect(notification.classList.contains('show')).toBe(true);
    expect(document.body.contains(notification)).toBe(true);
  });

  it('should remove element after fade transition', () => {
    const notification = showNotification('Test message');

    vi.advanceTimersByTime(3000);

    // Simulate transitionend event
    const transitionEndEvent = new Event('transitionend', { bubbles: true });
    notification.dispatchEvent(transitionEndEvent);

    expect(document.body.contains(notification)).toBe(false);
  });

  it('should handle multiple notifications', () => {
    const notification1 = showNotification('Message 1');
    const notification2 = showNotification('Message 2');

    expect(document.body.contains(notification1)).toBe(true);
    expect(document.body.contains(notification2)).toBe(true);
    expect(notification1.textContent).toContain('Message 1');
    expect(notification2.textContent).toContain('Message 2');
  });

  it('should have correct CSS classes', () => {
    const notification = showNotification('Test message');

    expect(notification.className).toContain('alert');
    expect(notification.className).toContain('alert-dismissible');
    expect(notification.className).toContain('fade');
    expect(notification.className).toContain('show');
    expect(notification.className).toContain('position-fixed');
  });

  it('should escape HTML in message', () => {
    const notification = showNotification('<script>alert("xss")</script>');

    // The message should be inserted as text, not HTML
    expect(notification.innerHTML).toContain('&lt;script&gt;');
  });
});
