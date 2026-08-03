import { afterEach, describe, expect, it, vi } from 'vitest';
import { createBaseline } from '../baselines';
import {
  createCategory,
  deleteCategory,
} from '../catalog';
import { createGroup, deleteGroup } from '../groups';
import {
  getNotifications,
  getUnreadCount,
  markAllNotificationsRead,
  markNotificationRead,
} from '../notifications';
import { listProjectsOptional } from '../projects';
import { listUsersOptional } from '../users';
import { createVerification, deleteVerificationGlobally } from '../verifications';
import { lastFetchCall, mockFetchFail, mockFetchOk } from './fetchTestUtils';

describe('optional list helpers', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it('listUsersOptional returns users or null on error', async () => {
    vi.stubGlobal('fetch', mockFetchOk([{ id: 1, username: 'a' }]));
    await expect(listUsersOptional()).resolves.toEqual([{ id: 1, username: 'a' }]);
    vi.stubGlobal('fetch', mockFetchFail(403, { message: 'nope' }));
    await expect(listUsersOptional()).resolves.toBeNull();
  });

  it('listProjectsOptional returns projects or null on error', async () => {
    vi.stubGlobal('fetch', mockFetchOk([{ id: 1, name: 'P', slug: 'p' }]));
    await expect(listProjectsOptional()).resolves.toHaveLength(1);
    vi.stubGlobal('fetch', mockFetchFail(403, { message: 'nope' }));
    await expect(listProjectsOptional()).resolves.toBeNull();
  });
});

describe('notifications api', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it('getNotifications builds query params', async () => {
    vi.stubGlobal('fetch', mockFetchOk([]));
    await getNotifications(true, 10);
    expect(lastFetchCall()[0]).toBe('/api/notifications?unread_only=true&limit=10');
    await getNotifications();
    expect(lastFetchCall()[0]).toBe('/api/notifications?limit=50');
  });

  it('getUnreadCount unwraps count', async () => {
    vi.stubGlobal('fetch', mockFetchOk({ count: 3 }));
    await expect(getUnreadCount()).resolves.toBe(3);
  });

  it('mark read endpoints use CSRF-only headers', async () => {
    vi.stubGlobal('fetch', mockFetchOk());
    await markNotificationRead(5, 'csrf');
    expect(lastFetchCall()[1]?.headers).toEqual({ 'X-CSRF-Token': 'csrf' });
    await markAllNotificationsRead('csrf');
    expect(lastFetchCall()[0]).toBe('/api/notifications/read-all');
  });
});

describe('baselines createBaseline', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it('trims description and sends null when empty', async () => {
    vi.stubGlobal('fetch', mockFetchOk({ id: 1, name: 'B' }));
    await createBaseline(1, 'B', '  ', 'csrf');
    expect(JSON.parse(String(lastFetchCall()[1]?.body))).toEqual({
      name: 'B',
      description: null,
    });
    await createBaseline(1, 'B', '  note  ', 'csrf');
    expect(JSON.parse(String(lastFetchCall()[1]?.body))).toEqual({
      name: 'B',
      description: 'note',
    });
  });
});

describe('sample CRUD shape', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it('catalog create/delete use JSON+CSRF and CSRF-only respectively', async () => {
    vi.stubGlobal('fetch', mockFetchOk({ id: 9 }));
    await createCategory(
      { title: 'C', tag: 'c', description: '', project_id: 1 },
      'csrf',
    );
    expect(lastFetchCall()[1]?.headers).toMatchObject({
      'Content-Type': 'application/json',
      'X-CSRF-Token': 'csrf',
    });
    vi.stubGlobal('fetch', mockFetchOk());
    await deleteCategory(9, 'csrf');
    expect(lastFetchCall()[1]?.headers).toEqual({ 'X-CSRF-Token': 'csrf' });
  });

  it('groups create/delete and verifications create/delete match header shapes', async () => {
    vi.stubGlobal('fetch', mockFetchOk({ id: 1, name: 'G', slug: 'g' }));
    await createGroup({ name: 'G' }, 'csrf');
    expect(lastFetchCall()[0]).toBe('/api/groups');
    vi.stubGlobal('fetch', mockFetchOk());
    await deleteGroup(1, 'csrf');
    expect(lastFetchCall()[1]?.headers).toEqual({ 'X-CSRF-Token': 'csrf' });

    vi.stubGlobal('fetch', mockFetchOk({ id: 8 }));
    await createVerification(
      {
        reference_code: 'V-1',
        name: 'V',
        description: 'd',
        source: '',
        status_id: 1,
        parent_id: null,
        project_id: 1,
        verification_method_id: null,
        author_id: 1,
        reviewer_id: 1,
      },
      'csrf',
    );
    expect(lastFetchCall()[0]).toBe('/api/verifications');
    vi.stubGlobal('fetch', mockFetchOk());
    await deleteVerificationGlobally(8, 'csrf');
    expect(lastFetchCall()[1]?.method).toBe('DELETE');
  });
});
