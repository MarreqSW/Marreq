import { describe, expect, it } from 'vitest';
import { formatUserLabel } from '../userLabel';

describe('formatUserLabel', () => {
  it('uses project members when the admin user list is unavailable', () => {
    expect(
      formatUserLabel(7, {
        users: null,
        members: [{ user_id: 7, name: 'Guest', username: 'guest' }],
      }),
    ).toBe('Guest (guest)');
  });

  it('prefers the admin directory when both sources have the id', () => {
    expect(
      formatUserLabel(1, {
        members: [{ user_id: 1, name: 'Alice', username: 'alice' }],
        users: [{ id: 1, name: 'Alice Johnson', username: 'alice' }],
      }),
    ).toBe('Alice Johnson (alice)');
  });

  it('falls back to the session user', () => {
    expect(
      formatUserLabel(7, {
        me: { id: 7, name: 'Guest', username: 'guest' },
      }),
    ).toBe('Guest (guest)');
  });

  it('returns a placeholder when the id is unknown', () => {
    expect(formatUserLabel(9, { users: null, members: [] })).toBe('User #9');
  });
});
