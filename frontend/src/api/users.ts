import type { User } from './types';
import { fetchJson } from './transport';

/** Admin-only; returns null if forbidden. */
export async function listUsersOptional(): Promise<User[] | null> {
  try {
    return await fetchJson<User[]>('/api/users');
  } catch {
    return null;
  }
}
