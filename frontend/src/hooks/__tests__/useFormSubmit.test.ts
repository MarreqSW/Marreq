import { act, renderHook } from '@testing-library/react';
import type { FormEvent } from 'react';
import { describe, expect, it, vi } from 'vitest';
import { useFormSubmit } from '../useFormSubmit';

function fakeEvent(): FormEvent {
  return {
    preventDefault: vi.fn(),
  } as unknown as FormEvent;
}

describe('useFormSubmit', () => {
  it('runs the action and clears submitting on success', async () => {
    const action = vi.fn().mockResolvedValue(undefined);
    const { result } = renderHook(() => useFormSubmit(action));

    await act(async () => {
      await result.current.onSubmit(fakeEvent());
    });

    expect(action).toHaveBeenCalled();
    expect(result.current.error).toBeNull();
    expect(result.current.submitting).toBe(false);
  });

  it('captures Error messages into error state', async () => {
    const action = vi.fn().mockRejectedValue(new Error('boom'));
    const { result } = renderHook(() => useFormSubmit(action));

    await act(async () => {
      await result.current.onSubmit(fakeEvent());
    });

    expect(result.current.error).toBe('boom');
    expect(result.current.submitting).toBe(false);
  });

  it('uses a fallback message for non-Error throws', async () => {
    const action = vi.fn().mockRejectedValue('nope');
    const { result } = renderHook(() => useFormSubmit(action));

    await act(async () => {
      await result.current.onSubmit(fakeEvent());
    });

    expect(result.current.error).toBe('An unexpected error occurred');
  });

  it('allows clearing error via setError', async () => {
    const action = vi.fn().mockRejectedValue(new Error('x'));
    const { result } = renderHook(() => useFormSubmit(action));

    await act(async () => {
      await result.current.onSubmit(fakeEvent());
    });
    expect(result.current.error).toBe('x');

    act(() => {
      result.current.setError(null);
    });
    expect(result.current.error).toBeNull();
  });
});
