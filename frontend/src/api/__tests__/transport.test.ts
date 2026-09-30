import { afterEach, describe, expect, it, vi } from 'vitest';
import { ApiError, fetchDownload, fetchJson, filenameFromDisposition, JSON_HEADERS } from '../transport';

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

  it('throws ApiError carrying the HTTP status and API message', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: false,
        status: 410,
        statusText: 'Gone',
        text: async () =>
          JSON.stringify({ status: 410, error: 'Gone', message: 'users self-register' }),
      }),
    );

    const err = await fetchJson('/api/users', { method: 'POST' }).catch((e: unknown) => e);
    expect(err).toBeInstanceOf(ApiError);
    expect(err).toBeInstanceOf(Error);
    expect((err as ApiError).status).toBe(410);
    expect((err as ApiError).message).toBe('users self-register');
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

describe('fetchDownload', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('returns the blob and the Content-Disposition filename', async () => {
    const blob = new Blob(['dump']);
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: true,
        status: 200,
        statusText: 'OK',
        headers: new Headers({
          'Content-Disposition': 'attachment; filename="marreq-backup_20260926_101500.sql.gz"',
        }),
        blob: async () => blob,
      }),
    );

    const res = await fetchDownload('/api/admin/backup', { method: 'POST' });
    expect(res.blob).toBe(blob);
    expect(res.filename).toBe('marreq-backup_20260926_101500.sql.gz');
  });

  it('throws ApiError on failure', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: false,
        status: 410,
        statusText: 'Gone',
        text: async () => JSON.stringify({ message: 'Database backup is not available' }),
      }),
    );

    const err = await fetchDownload('/api/admin/backup').catch((e: unknown) => e);
    expect(err).toBeInstanceOf(ApiError);
    expect((err as ApiError).status).toBe(410);
  });
});

describe('filenameFromDisposition', () => {
  it('parses quoted and unquoted filenames', () => {
    expect(filenameFromDisposition('attachment; filename="a.sql.gz"')).toBe('a.sql.gz');
    expect(filenameFromDisposition('attachment; filename=b.json')).toBe('b.json');
    expect(filenameFromDisposition(null)).toBeNull();
    expect(
      filenameFromDisposition(
        "attachment; filename=\"Gr__e.pdf\"; filename*=UTF-8''Gr%C3%B6%C3%9Fe%20v2.pdf",
      ),
    ).toBe('Größe v2.pdf');
  });
});
