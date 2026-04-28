import type { User } from '@/api/types';

/**
 * Narrow the loosely-typed `dashboard.user` payload (`unknown` on the wire,
 * see `DashboardPayload.user`) into a {@link User} when it has the expected
 * shape, otherwise return `null`.
 *
 * The dashboard endpoint can legitimately return `null`, an unauthenticated
 * placeholder, or a richer object than {@link User}, so callers always need
 * to guard before consuming user fields.
 */
export function parseUser(u: unknown): User | null {
  if (u && typeof u === 'object' && 'username' in u) return u as User;
  return null;
}
