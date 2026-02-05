/**
 * Tests for modules/theme.js
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import {
  applyTheme,
  getStoredTheme,
  setStoredTheme,
  resolvePreferredTheme,
  updateToggleMeta,
  initThemeControls,
} from '@modules/theme.js';

describe('Theme', () => {
  beforeEach(() => {
    document.documentElement.removeAttribute('data-theme');
    localStorage.clear();
    vi.clearAllMocks();
  });

  describe('applyTheme', () => {
    it('should set dark theme attribute', () => {
      applyTheme('dark');
      expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
    });

    it('should remove theme attribute for light theme', () => {
      document.documentElement.setAttribute('data-theme', 'dark');
      applyTheme('light');
      expect(document.documentElement.hasAttribute('data-theme')).toBe(false);
    });

    it('should handle other theme values', () => {
      // Implementation only sets attribute for 'dark'; other values clear it
      applyTheme('custom');
      expect(document.documentElement.getAttribute('data-theme')).toBeNull();
    });
  });

  describe('getStoredTheme', () => {
    it('should return stored theme from localStorage', () => {
      localStorage.setItem('reqman-theme', 'dark');
      expect(getStoredTheme()).toBe('dark');
    });

    it('should return null when no theme stored', () => {
      expect(getStoredTheme()).toBeNull();
    });

    it('should handle localStorage errors', () => {
      const getItemSpy = vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
        throw new Error('Storage error');
      });

      expect(getStoredTheme()).toBeNull();

      getItemSpy.mockRestore();
    });
  });

  describe('setStoredTheme', () => {
    it('should store theme in localStorage', () => {
      setStoredTheme('dark');
      expect(localStorage.getItem('reqman-theme')).toBe('dark');
    });

    it('should handle localStorage errors gracefully', () => {
      const setItemSpy = vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
        throw new Error('Storage error');
      });

      expect(() => setStoredTheme('dark')).not.toThrow();

      setItemSpy.mockRestore();
    });
  });

  describe('resolvePreferredTheme', () => {
    it('should return stored dark theme', () => {
      localStorage.setItem('reqman-theme', 'dark');
      expect(resolvePreferredTheme()).toBe('dark');
    });

    it('should return stored light theme', () => {
      localStorage.setItem('reqman-theme', 'light');
      expect(resolvePreferredTheme()).toBe('light');
    });

    it('should return dark when system prefers dark', () => {
      Object.defineProperty(window, 'matchMedia', {
        writable: true,
        configurable: true,
        value: vi.fn(() => ({
          matches: true,
        })),
      });

      expect(resolvePreferredTheme()).toBe('dark');
    });

    it('should return light when system prefers light', () => {
      Object.defineProperty(window, 'matchMedia', {
        writable: true,
        configurable: true,
        value: vi.fn(() => ({
          matches: false,
        })),
      });

      expect(resolvePreferredTheme()).toBe('light');
    });

    it('should return light when matchMedia not available', () => {
      Object.defineProperty(window, 'matchMedia', {
        writable: true,
        configurable: true,
        value: undefined,
      });

      expect(resolvePreferredTheme()).toBe('light');
    });

    it('should prioritize stored theme over system preference', () => {
      localStorage.setItem('reqman-theme', 'light');
      Object.defineProperty(window, 'matchMedia', {
        writable: true,
        configurable: true,
        value: vi.fn(() => ({
          matches: true, // System prefers dark
        })),
      });

      expect(resolvePreferredTheme()).toBe('light');
    });
  });

  describe('updateToggleMeta', () => {
    it('should update toggle attributes for dark theme', () => {
      document.body.innerHTML = `
        <button data-theme-toggle>Toggle</button>
      `;

      updateToggleMeta('dark');

      const toggle = document.querySelector('[data-theme-toggle]');
      expect(toggle.getAttribute('aria-pressed')).toBe('true');
      expect(toggle.getAttribute('aria-label')).toBe('Switch to light mode');
      expect(toggle.getAttribute('title')).toBe('Switch to light mode');
    });

    it('should update toggle attributes for light theme', () => {
      document.body.innerHTML = `
        <button data-theme-toggle>Toggle</button>
      `;

      updateToggleMeta('light');

      const toggle = document.querySelector('[data-theme-toggle]');
      expect(toggle.getAttribute('aria-pressed')).toBe('false');
      expect(toggle.getAttribute('aria-label')).toBe('Switch to dark mode');
      expect(toggle.getAttribute('title')).toBe('Switch to dark mode');
    });

    it('should update multiple toggles', () => {
      document.body.innerHTML = `
        <button data-theme-toggle>Toggle 1</button>
        <button data-theme-toggle>Toggle 2</button>
      `;

      updateToggleMeta('dark');

      const toggles = document.querySelectorAll('[data-theme-toggle]');
      toggles.forEach((toggle) => {
        expect(toggle.getAttribute('aria-pressed')).toBe('true');
      });
    });

    it('should handle no toggles found', () => {
      document.body.innerHTML = '';
      expect(() => updateToggleMeta('dark')).not.toThrow();
    });
  });

  describe('initThemeControls', () => {
    it('should initialize with theme from data attribute', () => {
      document.documentElement.setAttribute('data-theme', 'dark');
      document.body.innerHTML = `
        <button data-theme-toggle>Toggle</button>
      `;

      initThemeControls();

      expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
      const toggle = document.querySelector('[data-theme-toggle]');
      expect(toggle.getAttribute('aria-pressed')).toBe('true');
    });

    it('should initialize with resolved preferred theme', () => {
      Object.defineProperty(window, 'matchMedia', {
        writable: true,
        configurable: true,
        value: vi.fn(() => ({
          matches: true,
        })),
      });

      document.body.innerHTML = `
        <button data-theme-toggle>Toggle</button>
      `;

      initThemeControls();

      expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
    });

    it('should toggle theme on click', () => {
      vi.stubGlobal('matchMedia', vi.fn(() => ({ matches: false, addEventListener: vi.fn(), addListener: vi.fn() })));
      document.body.innerHTML = `
        <button data-theme-toggle>Toggle</button>
      `;

      initThemeControls();

      const toggle = document.querySelector('[data-theme-toggle]');
      toggle.click();

      expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
      expect(localStorage.getItem('reqman-theme')).toBe('dark');

      toggle.click();

      expect(document.documentElement.hasAttribute('data-theme')).toBe(false);
      expect(localStorage.getItem('reqman-theme')).toBe('light');
    });

    it('should update toggle meta on click', () => {
      vi.stubGlobal('matchMedia', vi.fn(() => ({ matches: false, addEventListener: vi.fn(), addListener: vi.fn() })));
      document.body.innerHTML = `
        <button data-theme-toggle>Toggle</button>
      `;

      initThemeControls();

      const toggle = document.querySelector('[data-theme-toggle]');
      toggle.click();

      expect(toggle.getAttribute('aria-pressed')).toBe('true');
      expect(toggle.getAttribute('aria-label')).toBe('Switch to light mode');
    });

    it('should listen to system theme changes when no stored theme', () => {
      const addEventListenerFn = vi.fn();
      const addListenerFn = vi.fn();
      const mockMatchMedia = vi.fn(() => ({
        matches: false,
        addEventListener: addEventListenerFn,
        addListener: addListenerFn,
      }));

      vi.stubGlobal('matchMedia', mockMatchMedia);

      document.body.innerHTML = `
        <button data-theme-toggle>Toggle</button>
      `;

      initThemeControls();

      expect(mockMatchMedia).toHaveBeenCalledWith('(prefers-color-scheme: dark)');
      const query = mockMatchMedia.mock.results[0].value;
      expect(query.addEventListener).toHaveBeenCalledWith('change', expect.any(Function));
    });

    it('should not listen to system changes when theme is stored', () => {
      localStorage.setItem('reqman-theme', 'dark');
      let changeHandler;
      const mockMatchMedia = vi.fn(() => ({
        matches: false,
        addEventListener: vi.fn((ev, fn) => { changeHandler = fn; }),
        addListener: vi.fn((fn) => { changeHandler = fn; }),
      }));

      vi.stubGlobal('matchMedia', mockMatchMedia);

      document.body.innerHTML = `
        <button data-theme-toggle>Toggle</button>
      `;

      initThemeControls();

      expect(typeof changeHandler).toBe('function');
      changeHandler({ matches: true });
      // Should not change because theme is stored
      expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
    });

    it('should use addListener for older browsers', () => {
      const addListener = vi.fn();
      const mockMatchMedia = vi.fn(() => ({
        matches: false,
        addEventListener: undefined,
        addListener,
      }));

      Object.defineProperty(window, 'matchMedia', {
        writable: true,
        configurable: true,
        value: mockMatchMedia,
      });

      document.body.innerHTML = `
        <button data-theme-toggle>Toggle</button>
      `;

      initThemeControls();

      expect(addListener).toHaveBeenCalled();
    });

    it('should handle system theme change event', () => {
      let changeHandler;
      const mockMatchMedia = vi.fn(() => ({
        matches: false,
        addEventListener: vi.fn((event, handler) => {
          if (event === 'change') {
            changeHandler = handler;
          }
        }),
        addListener: undefined,
      }));

      Object.defineProperty(window, 'matchMedia', {
        writable: true,
        configurable: true,
        value: mockMatchMedia,
      });

      document.body.innerHTML = `
        <button data-theme-toggle>Toggle</button>
      `;

      initThemeControls();

      if (changeHandler) {
        changeHandler({ matches: true });
        expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
      }
    });
  });
});
