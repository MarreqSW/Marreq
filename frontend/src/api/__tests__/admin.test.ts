import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { CSRF, itSendsEachRequest, lastRequest, stubFetch } from '@/test/apiHarness';
import { triggerDownload } from '@/utils/tableUtils';
import {
  createGroup,
  deleteGroup,
  getGroup,
  listCreatableGroups,
  listGroupMembers,
  listGroupProjects,
  listGroups,
  removeGroupMember,
  setGroupMemberRole,
  updateGroup,
} from '../groups';
import {
  cleanupAdminLogs,
  downloadAdminLogsJson,
  downloadDatabaseBackup,
  getAdminLogStats,
  listAdminLogs,
} from '../logs';
import { createUser, deleteUser, listUsersOptional, setUserPassword, updateUser } from '../users';

vi.mock('@/utils/tableUtils', () => ({ triggerDownload: vi.fn() }));

describe('groups API', () => {
  afterEach(() => vi.unstubAllGlobals());

  itSendsEachRequest([
    { name: 'listGroups', call: () => listGroups(), method: 'GET', url: '/api/groups' },
    { name: 'listCreatableGroups', call: () => listCreatableGroups(), method: 'GET', url: '/api/groups/creatable' },
    { name: 'getGroup', call: () => getGroup(4), method: 'GET', url: '/api/groups/4' },
    {
      name: 'createGroup',
      call: () => createGroup({ name: 'Payload', description: null }, CSRF),
      method: 'POST',
      url: '/api/groups',
      body: { name: 'Payload', description: null },
    },
    {
      name: 'updateGroup',
      call: () => updateGroup(4, { name: 'Payload team' }, CSRF),
      method: 'PATCH',
      url: '/api/groups/4',
      body: { name: 'Payload team' },
    },
    { name: 'deleteGroup', call: () => deleteGroup(4, CSRF), method: 'DELETE', url: '/api/groups/4' },
    { name: 'listGroupProjects', call: () => listGroupProjects(4), method: 'GET', url: '/api/groups/4/projects' },
    { name: 'listGroupMembers', call: () => listGroupMembers(4), method: 'GET', url: '/api/groups/4/members' },
    {
      name: 'setGroupMemberRole',
      call: () => setGroupMemberRole(4, 2, 1, CSRF),
      method: 'PUT',
      url: '/api/groups/4/members/2',
      body: { role: 1 },
    },
    {
      name: 'removeGroupMember',
      call: () => removeGroupMember(4, 2, CSRF),
      method: 'DELETE',
      url: '/api/groups/4/members/2',
    },
  ]);
});

describe('users API', () => {
  afterEach(() => vi.unstubAllGlobals());

  itSendsEachRequest([
    {
      name: 'listUsersOptional returns the list',
      call: () => listUsersOptional(),
      method: 'GET',
      url: '/api/users',
      response: { json: [{ id: 1 }] },
      returns: [{ id: 1 }],
    },
    {
      name: 'createUser returns the new id',
      call: () => createUser({ username: 'bob', is_admin: false } as never, CSRF),
      method: 'POST',
      url: '/api/users',
      body: { username: 'bob', is_admin: false },
      response: { json: { id: 7, username: 'bob' } },
      returns: { id: 7 },
    },
    {
      name: 'updateUser',
      call: () => updateUser(7, { is_admin: true } as never, CSRF),
      method: 'PUT',
      url: '/api/users/7',
      body: { is_admin: true },
    },
    {
      name: 'setUserPassword',
      call: () => setUserPassword(7, 'NewPass123!', 'NewPass123!', CSRF),
      method: 'PUT',
      url: '/api/users/7/password',
      body: { new_password: 'NewPass123!', confirm_password: 'NewPass123!' },
    },
    { name: 'deleteUser', call: () => deleteUser(7, CSRF), method: 'DELETE', url: '/api/users/7' },
  ]);

  it('listUsersOptional returns null for non-admins', async () => {
    stubFetch({ status: 403, json: { message: 'forbidden' } });
    await expect(listUsersOptional()).resolves.toBeNull();
  });
});

describe('admin logs API', () => {
  beforeEach(() => vi.mocked(triggerDownload).mockClear());
  afterEach(() => vi.unstubAllGlobals());

  itSendsEachRequest([
    { name: 'listAdminLogs without filters', call: () => listAdminLogs(), method: 'GET', url: '/api/admin/logs' },
    {
      name: 'listAdminLogs skips empty filters',
      call: () =>
        listAdminLogs({
          entity_type: 'PROJECT',
          entity_id: 5,
          user_id: 1,
          action_type: '',
          project_id: undefined,
          since: '2026-10-01',
          until: '2026-10-02',
          limit: 50,
          offset: 100,
        } as never),
      method: 'GET',
      url: '/api/admin/logs?entity_type=PROJECT&entity_id=5&user_id=1&since=2026-10-01&until=2026-10-02&limit=50&offset=100',
    },
    {
      name: 'getAdminLogStats passes top',
      call: () => getAdminLogStats({ project_id: 5, top: 3 } as never),
      method: 'GET',
      url: '/api/admin/logs/stats?project_id=5&top=3',
    },
    {
      name: 'cleanupAdminLogs',
      call: () => cleanupAdminLogs(90, CSRF),
      method: 'POST',
      url: '/api/admin/logs/cleanup',
      body: { days: 90 },
      response: { json: { deleted: 12 } },
      returns: { deleted: 12 },
    },
  ]);

  it('downloadAdminLogsJson saves the filtered export', async () => {
    const fetchMock = stubFetch({ text: '[]' });
    await downloadAdminLogsJson({ action_type: 'DELETE' } as never);
    expect(lastRequest(fetchMock).url).toBe('/api/admin/logs/export.json?action_type=DELETE');
    expect(triggerDownload).toHaveBeenCalledWith(expect.any(Blob), 'audit-logs.json');
  });

  it('downloadDatabaseBackup posts with CSRF and uses the server filename', async () => {
    const fetchMock = stubFetch({
      text: 'dump',
      headers: { 'Content-Disposition': 'attachment; filename="marreq-2026-10-02.sql.gz"' },
    });
    await expect(downloadDatabaseBackup(CSRF)).resolves.toBe('marreq-2026-10-02.sql.gz');
    const req = lastRequest(fetchMock);
    expect([req.method, req.url, req.headers['X-CSRF-Token']]).toEqual(['POST', '/api/admin/backup', CSRF]);
    expect(triggerDownload).toHaveBeenCalledWith(expect.any(Blob), 'marreq-2026-10-02.sql.gz');
  });

  it('downloadDatabaseBackup falls back to a default filename', async () => {
    stubFetch({ text: 'dump' });
    await expect(downloadDatabaseBackup(CSRF)).resolves.toBe('marreq-backup.sql.gz');
  });
});
