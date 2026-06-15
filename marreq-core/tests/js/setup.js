/**
 * Global test setup for Vitest
 */

import '@testing-library/jest-dom/vitest';

// Keep reference to real console before mocking to avoid recursion
const realConsole = { ...console };

// Mock console methods to reduce test noise
global.console = {
  ...console,
  log: vi.fn(),
  debug: vi.fn(),
  info: vi.fn(),
  warn: vi.fn(),
  error: (...args) => {
    realConsole.error(...args);
  },
};

// Mock localStorage
class LocalStorageMock {
  constructor() {
    this.store = {};
  }

  clear() {
    this.store = {};
  }

  getItem(key) {
    return this.store[key] || null;
  }

  setItem(key, value) {
    this.store[key] = String(value);
  }

  removeItem(key) {
    delete this.store[key];
  }

  key(index) {
    const keys = Object.keys(this.store);
    return keys[index] || null;
  }

  get length() {
    return Object.keys(this.store).length;
  }
}

global.localStorage = new LocalStorageMock();
global.sessionStorage = new LocalStorageMock();

// Reset before each test
beforeEach(() => {
  localStorage.clear();
  sessionStorage.clear();
});
