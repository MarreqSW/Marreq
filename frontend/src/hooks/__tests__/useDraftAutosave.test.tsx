import { act, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { DRAFT_DEBOUNCE_MS, useDraftAutosave } from '../useDraftAutosave';
import { readDraft } from '@/utils/requirementDraft';

const KEY = 'marreq-draft:v1:u7:p3:12';

type Props = { key: string | null; dirty: boolean; title: string; paused?: boolean };

function setup(initial: Props) {
  return renderHook(
    (p: Props) =>
      useDraftAutosave({
        key: p.key,
        dirty: p.dirty,
        values: { title: p.title },
        baseVersionId: 40,
        paused: p.paused,
      }),
    { initialProps: initial },
  );
}

describe('useDraftAutosave (issue #255)', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });
  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
    localStorage.clear();
  });

  it('writes the draft once typing pauses', () => {
    const { result, rerender } = setup({ key: KEY, dirty: true, title: 'A' });
    expect(result.current.status).toBe('pending');
    rerender({ key: KEY, dirty: true, title: 'AB' });
    act(() => vi.advanceTimersByTime(DRAFT_DEBOUNCE_MS - 1));
    expect(readDraft(KEY)).toBeNull();
    act(() => vi.advanceTimersByTime(1));
    expect(readDraft(KEY)).toMatchObject({ baseVersionId: 40, values: { title: 'AB' } });
    expect(result.current.status).toBe('saved');
    expect(result.current.savedAt).toBeInstanceOf(Date);
  });

  it('removes the draft when the form is clean again', () => {
    const { result, rerender } = setup({ key: KEY, dirty: true, title: 'A' });
    act(() => vi.advanceTimersByTime(DRAFT_DEBOUNCE_MS));
    expect(readDraft(KEY)).not.toBeNull();
    rerender({ key: KEY, dirty: false, title: '' });
    expect(readDraft(KEY)).toBeNull();
    expect(result.current.status).toBe('clean');
  });

  it('writes nothing without a key or while paused', () => {
    const { rerender } = setup({ key: null, dirty: true, title: 'A' });
    act(() => vi.advanceTimersByTime(DRAFT_DEBOUNCE_MS));
    rerender({ key: KEY, dirty: true, title: 'A', paused: true });
    act(() => vi.advanceTimersByTime(DRAFT_DEBOUNCE_MS));
    expect(readDraft(KEY)).toBeNull();
  });

  it('writes a pending change at once when the page is hidden or left', () => {
    const { rerender } = setup({ key: KEY, dirty: true, title: 'A' });
    Object.defineProperty(document, 'visibilityState', { configurable: true, value: 'hidden' });
    act(() => {
      document.dispatchEvent(new Event('visibilitychange'));
    });
    expect(readDraft(KEY)).toMatchObject({ values: { title: 'A' } });

    rerender({ key: KEY, dirty: true, title: 'AB' });
    act(() => {
      window.dispatchEvent(new Event('pagehide'));
    });
    expect(readDraft(KEY)).toMatchObject({ values: { title: 'AB' } });
    Object.defineProperty(document, 'visibilityState', { configurable: true, value: 'visible' });
  });

  it('flushes on unmount, but not after discard', () => {
    const first = setup({ key: KEY, dirty: true, title: 'A' });
    first.unmount();
    expect(readDraft(KEY)).toMatchObject({ values: { title: 'A' } });

    const second = setup({ key: KEY, dirty: true, title: 'B' });
    act(() => second.result.current.discard());
    second.unmount();
    expect(readDraft(KEY)).toBeNull();
  });

  it('warns before leaving only when no draft could be kept', () => {
    const leave = () => {
      const event = new Event('beforeunload', { cancelable: true });
      window.dispatchEvent(event);
      return event.defaultPrevented;
    };
    const kept = setup({ key: KEY, dirty: true, title: 'A' });
    act(() => vi.advanceTimersByTime(DRAFT_DEBOUNCE_MS));
    expect(kept.result.current.status).toBe('saved');
    expect(leave()).toBe(false);
    kept.unmount();

    const setItem = vi.spyOn(localStorage, 'setItem').mockImplementation(() => {
      throw new DOMException('quota', 'QuotaExceededError');
    });
    try {
      const failing = setup({ key: KEY, dirty: true, title: 'B' });
      act(() => vi.advanceTimersByTime(DRAFT_DEBOUNCE_MS));
      expect(failing.result.current.status).toBe('error');
      expect(leave()).toBe(true);
      failing.unmount();
    } finally {
      setItem.mockRestore();
    }
  });
});
