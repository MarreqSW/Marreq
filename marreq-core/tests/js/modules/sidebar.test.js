/**
 * Tests for modules/sidebar.js
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { initSidebar } from '@modules/sidebar.js';

describe('Sidebar', () => {
  beforeEach(() => {
    document.body.innerHTML = '';
    localStorage.clear();
    // Mock window.innerWidth
    Object.defineProperty(window, 'innerWidth', {
      writable: true,
      configurable: true,
      value: 1200, // Desktop width
    });
  });

  it('should return early if sidebar not found', () => {
    expect(() => initSidebar()).not.toThrow();
  });

  it('should initialize sidebar with toggle button', () => {
    document.body.innerHTML = `
      <div id="mainSidebar"></div>
      <button id="sidebarToggle">Toggle</button>
    `;

    initSidebar();

    const sidebar = document.getElementById('mainSidebar');
    const toggle = document.getElementById('sidebarToggle');
    expect(sidebar).toBeTruthy();
    expect(toggle).toBeTruthy();
  });

  it('should restore collapsed state from localStorage on desktop', () => {
    localStorage.setItem('marreq_sidebar_collapsed', 'true');
    Object.defineProperty(window, 'innerWidth', { value: 1200, writable: true, configurable: true });

    document.body.innerHTML = `
      <div id="mainSidebar"></div>
      <button id="sidebarToggle">Toggle</button>
    `;

    initSidebar();

    const sidebar = document.getElementById('mainSidebar');
    expect(sidebar.classList.contains('marreq-sidebar--collapsed')).toBe(true);
  });

  it('should not restore collapsed state on mobile', () => {
    localStorage.setItem('marreq_sidebar_collapsed', 'true');
    Object.defineProperty(window, 'innerWidth', { value: 800, writable: true, configurable: true });

    document.body.innerHTML = `
      <div id="mainSidebar"></div>
      <button id="sidebarToggle">Toggle</button>
    `;

    initSidebar();

    const sidebar = document.getElementById('mainSidebar');
    expect(sidebar.classList.contains('marreq-sidebar--collapsed')).toBe(false);
  });

  it('should toggle sidebar on desktop toggle click', () => {
    document.body.innerHTML = `
      <div id="mainSidebar"></div>
      <button id="sidebarToggle">Toggle</button>
    `;

    initSidebar();

    const sidebar = document.getElementById('mainSidebar');
    const toggle = document.getElementById('sidebarToggle');

    expect(sidebar.classList.contains('marreq-sidebar--collapsed')).toBe(false);

    toggle.click();
    expect(sidebar.classList.contains('marreq-sidebar--collapsed')).toBe(true);

    toggle.click();
    expect(sidebar.classList.contains('marreq-sidebar--collapsed')).toBe(false);
  });

  it('should save collapsed state to localStorage on desktop', () => {
    document.body.innerHTML = `
      <div id="mainSidebar"></div>
      <button id="sidebarToggle">Toggle</button>
    `;

    initSidebar();

    const toggle = document.getElementById('sidebarToggle');
    toggle.click();

    expect(localStorage.getItem('marreq_sidebar_collapsed')).toBe('true');
  });

  it('should not save collapsed state to localStorage on mobile', () => {
    Object.defineProperty(window, 'innerWidth', { value: 800, writable: true, configurable: true });

    document.body.innerHTML = `
      <div id="mainSidebar"></div>
      <button id="sidebarToggle">Toggle</button>
    `;

    initSidebar();

    const toggle = document.getElementById('sidebarToggle');
    toggle.click();

    expect(localStorage.getItem('marreq_sidebar_collapsed')).toBeNull();
  });

  it('should handle mobile toggle button', () => {
    Object.defineProperty(window, 'innerWidth', { value: 800, writable: true, configurable: true });

    document.body.innerHTML = `
      <div id="mainSidebar"></div>
      <button id="mobileToggle">Mobile Toggle</button>
    `;

    initSidebar();

    const sidebar = document.getElementById('mainSidebar');
    const mobileToggle = document.getElementById('mobileToggle');

    expect(sidebar.classList.contains('marreq-sidebar--mobile-open')).toBe(false);

    mobileToggle.click();
    expect(sidebar.classList.contains('marreq-sidebar--mobile-open')).toBe(true);

    mobileToggle.click();
    expect(sidebar.classList.contains('marreq-sidebar--mobile-open')).toBe(false);
  });

  it('should close mobile sidebar when clicking outside', () => {
    Object.defineProperty(window, 'innerWidth', { value: 800, writable: true, configurable: true });

    document.body.innerHTML = `
      <div id="mainSidebar">
        <div>Sidebar content</div>
      </div>
      <button id="mobileToggle">Mobile Toggle</button>
      <div id="outside">Outside content</div>
    `;

    initSidebar();

    const sidebar = document.getElementById('mainSidebar');
    const mobileToggle = document.getElementById('mobileToggle');
    const outside = document.getElementById('outside');

    mobileToggle.click();
    expect(sidebar.classList.contains('marreq-sidebar--mobile-open')).toBe(true);

    outside.click();
    expect(sidebar.classList.contains('marreq-sidebar--mobile-open')).toBe(false);
  });

  it('should not close mobile sidebar when clicking inside sidebar', () => {
    Object.defineProperty(window, 'innerWidth', { value: 800, writable: true, configurable: true });

    document.body.innerHTML = `
      <div id="mainSidebar">
        <div id="inside">Sidebar content</div>
      </div>
      <button id="mobileToggle">Mobile Toggle</button>
    `;

    initSidebar();

    const sidebar = document.getElementById('mainSidebar');
    const mobileToggle = document.getElementById('mobileToggle');
    const inside = document.getElementById('inside');

    mobileToggle.click();
    expect(sidebar.classList.contains('marreq-sidebar--mobile-open')).toBe(true);

    inside.click();
    expect(sidebar.classList.contains('marreq-sidebar--mobile-open')).toBe(true);
  });

  it('should not close mobile sidebar when clicking mobile toggle', () => {
    Object.defineProperty(window, 'innerWidth', { value: 800, writable: true, configurable: true });

    document.body.innerHTML = `
      <div id="mainSidebar"></div>
      <button id="mobileToggle">Mobile Toggle</button>
    `;

    initSidebar();

    const sidebar = document.getElementById('mainSidebar');
    const mobileToggle = document.getElementById('mobileToggle');

    mobileToggle.click();
    expect(sidebar.classList.contains('marreq-sidebar--mobile-open')).toBe(true);

    // Clicking toggle again should close it (not prevented by outside click handler)
    mobileToggle.click();
    expect(sidebar.classList.contains('marreq-sidebar--mobile-open')).toBe(false);
  });

  it('should handle localStorage errors gracefully', () => {
    const getItemSpy = vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
      throw new Error('Storage error');
    });

    document.body.innerHTML = `
      <div id="mainSidebar"></div>
      <button id="sidebarToggle">Toggle</button>
    `;

    expect(() => initSidebar()).not.toThrow();

    getItemSpy.mockRestore();
  });

  it('should handle localStorage setItem errors gracefully', () => {
    const setItemSpy = vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('Storage error');
    });

    document.body.innerHTML = `
      <div id="mainSidebar"></div>
      <button id="sidebarToggle">Toggle</button>
    `;

    initSidebar();

    const toggle = document.getElementById('sidebarToggle');
    expect(() => toggle.click()).not.toThrow();

    setItemSpy.mockRestore();
  });

  it('should work without mobile toggle button', () => {
    document.body.innerHTML = `
      <div id="mainSidebar"></div>
      <button id="sidebarToggle">Toggle</button>
    `;

    expect(() => initSidebar()).not.toThrow();
  });

  it('should work without desktop toggle button', () => {
    document.body.innerHTML = `
      <div id="mainSidebar"></div>
      <button id="mobileToggle">Mobile Toggle</button>
    `;

    expect(() => initSidebar()).not.toThrow();
  });
});
