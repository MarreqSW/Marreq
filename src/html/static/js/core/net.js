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
    credentials: options.credentials || 'same-origin',
  };

  if (isJson(body)) {
    init.body = JSON.stringify(body);
    init.headers['Content-Type'] = init.headers['Content-Type'] || 'application/json';
  } else if (body instanceof FormData) {
    init.body = body;
  }

  const response = await fetch(url, init);

  let payload = null;
  if (parse) {
    const text = await response.text();
    payload = text ? JSON.parse(text) : null;
  }

  if (!response.ok || (payload && payload.success === false)) {
    const message = payload?.message || `Request to ${url} failed with status ${response.status}`;
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

