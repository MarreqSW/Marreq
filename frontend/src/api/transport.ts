const JSON_HEADERS = { 'Content-Type': 'application/json' };

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

async function parseJson<T>(res: Response): Promise<T> {
  const text = await res.text();
  if (!res.ok) {
    let msg = res.statusText;
    try {
      const j = JSON.parse(text) as { message?: string; error?: string };
      msg = (j.message ?? j.error ?? text) || msg;
    } catch {
      msg = friendlyNonJsonError(res.status, text);
    }
    throw new Error(msg);
  }
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
