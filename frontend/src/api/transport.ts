const JSON_HEADERS = { 'Content-Type': 'application/json' };

/** Marks automatic requests (polling) that must not count as user activity for the session idle limit. */
export const BACKGROUND_REQUEST_HEADER = 'X-Marreq-Background';

function friendlyNonJsonError(status: number, text: string): string {
  const t = text.trim();
  if (
    t.startsWith('<!DOCTYPE') ||
    t.startsWith('<html') ||
    t.toLowerCase().includes('<title>404 not found</title>')
  ) {
    return `Server returned ${status} with an HTML error page (typical of opening the API port directly). Use the Vite URL (e.g. http://127.0.0.1:5173) or the nginx frontend so /p/… routes load the React app; only /api/… should hit Rocket.`;
  }
  return t || `Request failed (${status})`;
}

/** Error thrown for non-2xx API responses; keeps the HTTP status for callers that branch on it. */
export class ApiError extends Error {
  readonly status: number;

  constructor(status: number, message: string) {
    super(message);
    this.name = 'ApiError';
    this.status = status;
  }
}

function apiErrorFrom(res: Response, text: string): ApiError {
  let msg = res.statusText;
  try {
    const j = JSON.parse(text) as { message?: string; error?: string };
    msg = (j.message ?? j.error ?? text) || msg;
  } catch {
    msg = friendlyNonJsonError(res.status, text);
  }
  return new ApiError(res.status, msg);
}

async function parseJson<T>(res: Response): Promise<T> {
  const text = await res.text();
  if (!res.ok) throw apiErrorFrom(res, text);
  if (!text) return undefined as T;
  return JSON.parse(text) as T;
}

export { JSON_HEADERS };

export async function fetchJson<T>(
  path: string,
  init: RequestInit = {},
): Promise<T> {
  const res = await fetch(path, {
    credentials: 'same-origin',
    ...init,
    headers: {
      ...init.headers,
    },
  });
  return parseJson<T>(res);
}

/**
 * Filename from a `Content-Disposition: attachment; filename="…"` header, if any.
 * Prefers the RFC 5987 `filename*=UTF-8''…` form, which keeps non-ASCII names.
 */
export function filenameFromDisposition(header: string | null): string | null {
  const encoded = header?.match(/filename\*=UTF-8''([^;]+)/i);
  if (encoded) {
    try {
      return decodeURIComponent(encoded[1].trim());
    } catch {
      // Fall back to the plain parameter below.
    }
  }
  const match = header?.match(/filename="?([^";]+)"?/i);
  return match ? match[1] : null;
}

/** Like {@link fetchBlob}, but also returns the server-suggested filename. */
export async function fetchDownload(
  path: string,
  init: RequestInit = {},
): Promise<{ blob: Blob; filename: string | null }> {
  const res = await fetch(path, {
    credentials: 'same-origin',
    ...init,
    headers: {
      ...init.headers,
    },
  });
  if (!res.ok) throw apiErrorFrom(res, await res.text());
  return {
    blob: await res.blob(),
    filename: filenameFromDisposition(res.headers.get('Content-Disposition')),
  };
}

/** Fetches a binary response (file download). Errors use the same messages as fetchJson. */
export async function fetchBlob(path: string, init: RequestInit = {}): Promise<Blob> {
  const res = await fetch(path, {
    credentials: 'same-origin',
    ...init,
    headers: {
      ...init.headers,
    },
  });
  if (!res.ok) throw apiErrorFrom(res, await res.text());
  return res.blob();
}
