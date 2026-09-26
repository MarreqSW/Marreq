import type { User, UserCreateRequest, UserUpdateRequest } from './types';
import { fetchJson, JSON_HEADERS } from './transport';

/** Admin-only; returns null if forbidden. */
export async function listUsersOptional(): Promise<User[] | null> {
  try {
    return await fetchJson<User[]>('/api/users');
  } catch {
    return null;
  }
}

/** Admin-only. Returns 410 Gone when the deployment only allows self-registration. */
export async function createUser(
  body: UserCreateRequest,
  csrfToken: string,
): Promise<{ id: number }> {
  const r = await fetchJson<{ id: number }>('/api/users', {
    method: 'POST',
    headers: { ...JSON_HEADERS, 'X-CSRF-Token': csrfToken },
    body: JSON.stringify(body),
  });
  return { id: r.id };
}

/** Admin-only. Updates profile fields and the admin flag. */
export async function updateUser(
  id: number,
  body: UserUpdateRequest,
  csrfToken: string,
): Promise<User> {
  return fetchJson<User>(`/api/users/${id}`, {
    method: 'PUT',
    headers: { ...JSON_HEADERS, 'X-CSRF-Token': csrfToken },
    body: JSON.stringify(body),
  });
}

/** Admin-only. Sets a password without the current one and signs the user out elsewhere. */
export async function setUserPassword(
  id: number,
  newPassword: string,
  confirmPassword: string,
  csrfToken: string,
): Promise<void> {
  await fetchJson(`/api/users/${id}/password`, {
    method: 'PUT',
    headers: { ...JSON_HEADERS, 'X-CSRF-Token': csrfToken },
    body: JSON.stringify({ new_password: newPassword, confirm_password: confirmPassword }),
  });
}

/** Admin-only. Refused for your own account and for the last administrator. */
export async function deleteUser(id: number, csrfToken: string): Promise<void> {
  await fetchJson(`/api/users/${id}`, {
    method: 'DELETE',
    headers: { 'X-CSRF-Token': csrfToken },
  });
}
