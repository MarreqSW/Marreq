function isJson(body) {
  return body !== undefined && body !== null && !(body instanceof FormData);
}

export async function jsonFetch(url, options = {}) {
  const {
    method = 'GET',
    headers = {},
    body,
    parse = true,
  } = options;

  const init = {
    method,
    headers: {
      Accept: 'application/json',
      ...headers,
    },
    // Use include so credentialed API calls behave consistently via Vite/nginx proxies (same as docs/developer/http-api-contract.md).
    credentials: options.credentials ?? 'include',
  };

  if (isJson(body)) {
    init.body = JSON.stringify(body);
    init.headers['Content-Type'] = init.headers['Content-Type'] || 'application/json';
  } else if (body instanceof FormData) {
    init.body = body;
  }

  const response = await fetch(url, init);

  let payload = null;
  let rawText = '';
  if (parse) {
    rawText = await response.text();
    if (rawText) {
      const trimmed = rawText.replace(/^\uFEFF/, '').trim();
      try {
        payload = JSON.parse(trimmed);
      } catch {
        // e.g. Rocket CSRF denial returns plain text "403 Forbidden – …" (starts with digits → JSON.parse throws).
        payload = null;
      }
    }
  }

  if (!response.ok || (payload && payload.success === false)) {
    const message =
      payload?.message ||
      (rawText && !payload ? rawText.trim().slice(0, 500) : null) ||
      `Request to ${url} failed with status ${response.status}`;
    const error = new Error(message);
    error.response = response;
    error.payload = payload;
    throw error;
  }

  return payload;
}

export const postJson = (url, body, options = {}) =>
  jsonFetch(url, { method: 'POST', body, ...options });

export const patchJson = (url, body, options = {}) =>
  jsonFetch(url, { method: 'PATCH', body, ...options });

export const deleteJson = (url, options = {}) =>
  jsonFetch(url, { method: 'DELETE', parse: options.parse ?? false, ...options });

export function formToJSON(form) {
  const formData = new FormData(form);
  return Object.fromEntries(formData.entries());
}
