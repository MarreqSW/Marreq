import { afterEach, describe, expect, it, vi } from 'vitest';
import { fetchJson, JSON_HEADERS } from '../transport';

describe('JSON_HEADERS', () => {
  it('sets application/json content type', () => {
    expect(JSON_HEADERS).toEqual({ 'Content-Type': 'application/json' });
  });
});

describe('fetchJson', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it('returns parsed JSON on success', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: true,
        status: 200,
        statusText: 'OK',
        text: async () => JSON.stringify({ id: 1, name: 'Alice' }),
      }),
    );

    const data = await fetchJson<{ id: number; name: string }>('/api/users/1');
    expect(data).toEqual({ id: 1, name: 'Alice' });
    expect(fetch).toHaveBeenCalledWith(
      '/api/users/1',
      expect.objectContaining({ credentials: 'same-origin' }),
    );
  });

  it('returns undefined for empty successful bodies', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: true,
        status: 204,
        statusText: 'No Content',
        text: async () => '',
      }),
    );

    await expect(fetchJson('/api/noop')).resolves.toBeUndefined();
  });

  it('throws using JSON message when present', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: false,
        status: 400,
        statusText: 'Bad Request',
        text: async () => JSON.stringify({ message: 'Invalid input' }),
      }),
    );

    await expect(fetchJson('/api/bad')).rejects.toThrow('Invalid input');
  });

  it('throws using JSON error field when message is absent', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: false,
        status: 403,
        statusText: 'Forbidden',
        text: async () => JSON.stringify({ error: 'not allowed' }),
      }),
    );

    await expect(fetchJson('/api/forbidden')).rejects.toThrow('not allowed');
  });

  it('maps HTML error pages to a friendly message', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: false,
        status: 404,
        statusText: 'Not Found',
        text: async () => '<!DOCTYPE html><html><title>404 Not Found</title></html>',
      }),
    );

    await expect(fetchJson('/missing')).rejects.toThrow(/HTML error page/i);
  });

  it('falls back to status text for non-JSON plain errors', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: false,
        status: 500,
        statusText: 'Internal Server Error',
        text: async () => 'boom',
      }),
    );

    await expect(fetchJson('/api/crash')).rejects.toThrow('boom');
  });
});
