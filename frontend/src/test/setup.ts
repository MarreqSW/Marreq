import '@testing-library/jest-dom/vitest';

// happy-dom >= 20.14 no longer defines the blocking dialog APIs. The app uses
// `window.confirm` (browsers always provide it), and tests stub it with
// `vi.spyOn(window, 'confirm')`, which needs an existing function to wrap.
// Defaults mirror a user dismissing the dialog.
const dialogDefaults: Record<'alert' | 'confirm' | 'prompt', (...args: unknown[]) => unknown> = {
  alert: () => undefined,
  confirm: () => false,
  prompt: () => null,
};
for (const [name, impl] of Object.entries(dialogDefaults)) {
  if (typeof (window as unknown as Record<string, unknown>)[name] !== 'function') {
    Object.defineProperty(window, name, { configurable: true, writable: true, value: impl });
  }
}
