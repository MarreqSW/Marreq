import { vi } from 'vitest';

export function mockFetchOk(body: unknown = {}, status = 200) {
  const text =
    body === undefined || body === ''
      ? ''
      : typeof body === 'string'
        ? body
        : JSON.stringify(body);
  return vi.fn().mockResolvedValue({
    ok: true,
    status,
    statusText: 'OK',
    text: async () => text,
    json: async () => (typeof body === 'string' ? JSON.parse(body) : body),
  });
}

export function mockFetchFail(status: number, body: unknown, statusText = 'Error') {
  const text = typeof body === 'string' ? body : JSON.stringify(body);
  return vi.fn().mockResolvedValue({
    ok: false,
    status,
    statusText,
    text: async () => text,
    json: async () => {
      throw new Error('no json');
    },
  });
}

export function lastFetchCall(): [string, RequestInit | undefined] {
  const fn = fetch as unknown as ReturnType<typeof vi.fn>;
  const call = fn.mock.calls.at(-1);
  if (!call) throw new Error('fetch was not called');
  return call as [string, RequestInit | undefined];
}
