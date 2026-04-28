import { describe, it, expect } from 'vitest';
import { parseUser } from '../parseUser';

describe('parseUser', () => {
  it('returns the object cast to User when it has a username', () => {
    const raw = { id: 1, username: 'alice', name: 'Alice', is_admin: true };
    const u = parseUser(raw);
    expect(u).not.toBeNull();
    expect(u?.username).toBe('alice');
    expect(u?.is_admin).toBe(true);
  });

  it('returns null for null', () => {
    expect(parseUser(null)).toBeNull();
  });

  it('returns null for undefined', () => {
    expect(parseUser(undefined)).toBeNull();
  });

  it('returns null for primitives', () => {
    expect(parseUser('alice')).toBeNull();
    expect(parseUser(42)).toBeNull();
    expect(parseUser(true)).toBeNull();
  });

  it('returns null for objects without a username key', () => {
    expect(parseUser({ id: 1 })).toBeNull();
    expect(parseUser({})).toBeNull();
  });
});
