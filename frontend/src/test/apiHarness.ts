import { expect, it, vi } from 'vitest';

/** CSRF token the endpoint tables pass to every write call. */
export const CSRF = 'csrf-token';

type StubResponse = {
  status?: number;
  /** JSON body; `undefined` sends an empty body (e.g. 204). */
  json?: unknown;
  /** Raw text body, used instead of `json`. */
  text?: string;
  headers?: Record<string, string>;
};

/** Replace `fetch` with a mock that answers every request with `response`. */
export function stubFetch(response: StubResponse = {}) {
  const status = response.status ?? 200;
  const text = response.text ?? (response.json === undefined ? '' : JSON.stringify(response.json));
  const fetchMock = vi.fn(async (_url: string, _init?: RequestInit) => ({
    ok: status >= 200 && status < 300,
    status,
    statusText: status >= 400 ? 'Error' : 'OK',
    headers: new Headers(response.headers ?? {}),
    text: async () => text,
    json: async () => JSON.parse(text),
    blob: async () => new Blob([text]),
  }));
  vi.stubGlobal('fetch', fetchMock);
  return fetchMock;
}

export type SentRequest = {
  url: string;
  method: string;
  headers: Record<string, string>;
  credentials: RequestCredentials | undefined;
  /** Parsed JSON body, the `FormData` entries as an object, or `undefined`. */
  body: unknown;
};

/** The last request sent through the mock from {@link stubFetch}. */
export function lastRequest(fetchMock: ReturnType<typeof stubFetch>): SentRequest {
  const call = fetchMock.mock.calls.at(-1);
  if (!call) throw new Error('fetch was not called');
  const [url, init = {}] = call;
  let body: unknown;
  if (typeof init.body === 'string') body = JSON.parse(init.body);
  else if (init.body instanceof FormData) {
    body = Object.fromEntries(
      [...init.body.entries()].map(([k, v]) => [k, typeof v === 'string' ? v : `file:${v.name}`]),
    );
  }
  return {
    url,
    method: init.method ?? 'GET',
    headers: (init.headers ?? {}) as Record<string, string>,
    credentials: init.credentials,
    body,
  };
}

export type EndpointCase = {
  name: string;
  call: () => Promise<unknown>;
  method: 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE';
  url: string;
  /** Expected JSON body, or the multipart fields (files as `file:<name>`). */
  body?: unknown;
  /** Whether a JSON `Content-Type` header is required (default: when there is a JSON body). */
  json?: boolean;
  /** Whether the request must carry the CSRF header (default: every non-GET request). */
  csrf?: boolean;
  /** What the server answers (default `{}`). */
  response?: StubResponse;
  /** What the function resolves to, when that matters. */
  returns?: unknown;
};

/**
 * One test per endpoint: the URL, method, CSRF and content-type headers and
 * body the SPA sends, and what the function returns.
 */
export function itSendsEachRequest(cases: EndpointCase[]) {
  it.each(cases.map((c) => [c.name, c] as const))('%s', async (_name, c) => {
    const fetchMock = stubFetch(c.response ?? { json: {} });
    const result = await c.call();
    const req = lastRequest(fetchMock);

    expect(req.url).toBe(c.url);
    expect(req.method).toBe(c.method);
    expect(req.credentials).toBe('same-origin');
    const csrf = c.csrf ?? c.method !== 'GET';
    if (csrf) expect(req.headers['X-CSRF-Token']).toBe(CSRF);
    else expect(req.headers['X-CSRF-Token']).toBeUndefined();
    if (c.body !== undefined) expect(req.body).toEqual(c.body);
    else expect(req.body).toBeUndefined();
    if (isMultipart(fetchMock)) {
      // The browser sets the multipart boundary; an explicit Content-Type would break it.
      expect(req.headers['Content-Type']).toBeUndefined();
    } else if (c.json ?? c.body !== undefined) {
      expect(req.headers['Content-Type']).toBe('application/json');
    }
    if (c.returns !== undefined) expect(result).toEqual(c.returns);
  });
}

function isMultipart(fetchMock: ReturnType<typeof stubFetch>): boolean {
  return fetchMock.mock.calls.at(-1)?.[1]?.body instanceof FormData;
}
