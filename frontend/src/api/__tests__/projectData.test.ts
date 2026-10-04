import { afterEach, describe, expect, it, vi } from 'vitest';
import { CSRF, itSendsEachRequest, lastRequest, stubFetch } from '@/test/apiHarness';
import { DEFAULT_DSM_PARAMS } from '@/utils/dsm';
import {
  createBaseline,
  getBaseline,
  getBaselineRequirements,
  getBaselineTraceability,
  getBaselineVerifications,
  listBaselines,
} from '../baselines';
import { getDashboard } from '../dashboard';
import {
  deleteNotificationPreference,
  getNotificationPreferences,
  getNotifications,
  getUnreadCount,
  markAllNotificationsRead,
  markNotificationRead,
  setNotificationPreference,
} from '../notifications';
import { createSavedView, deleteSavedView, getSavedView, listSavedViews, updateSavedView } from '../savedViews';
import { clearTraceabilitySuspect, getDsm } from '../traceability';

const view = { name: 'CDR scope', definition: { filters: {} } } as never;

describe('baselines API', () => {
  afterEach(() => vi.unstubAllGlobals());

  itSendsEachRequest([
    { name: 'listBaselines', call: () => listBaselines(5), method: 'GET', url: '/api/projects/5/baselines' },
    { name: 'getBaseline', call: () => getBaseline(5, 3), method: 'GET', url: '/api/projects/5/baselines/3' },
    {
      name: 'createBaseline trims the description and links the saved view',
      call: () => createBaseline(5, 'CDR', '  frozen for CDR  ', CSRF, 9),
      method: 'POST',
      url: '/api/projects/5/baselines',
      body: { name: 'CDR', description: 'frozen for CDR', saved_view_id: 9 },
    },
    {
      name: 'createBaseline sends nulls for a blank description and no view',
      call: () => createBaseline(5, 'PDR', '   ', CSRF),
      method: 'POST',
      url: '/api/projects/5/baselines',
      body: { name: 'PDR', description: null, saved_view_id: null },
    },
    {
      name: 'getBaselineRequirements',
      call: () => getBaselineRequirements(5, 3),
      method: 'GET',
      url: '/api/projects/5/baselines/3/requirements',
    },
    {
      name: 'getBaselineTraceability',
      call: () => getBaselineTraceability(5, 3),
      method: 'GET',
      url: '/api/projects/5/baselines/3/traceability',
    },
    {
      name: 'getBaselineVerifications',
      call: () => getBaselineVerifications(5, 3),
      method: 'GET',
      url: '/api/projects/5/baselines/3/verifications',
    },
  ]);
});

describe('saved views API', () => {
  afterEach(() => vi.unstubAllGlobals());

  itSendsEachRequest([
    { name: 'listSavedViews', call: () => listSavedViews(5), method: 'GET', url: '/api/projects/5/saved_views' },
    { name: 'getSavedView', call: () => getSavedView(5, 9), method: 'GET', url: '/api/projects/5/saved_views/9' },
    {
      name: 'createSavedView',
      call: () => createSavedView(5, view, CSRF),
      method: 'POST',
      url: '/api/projects/5/saved_views',
      body: view,
    },
    {
      name: 'updateSavedView',
      call: () => updateSavedView(5, 9, view, CSRF),
      method: 'PATCH',
      url: '/api/projects/5/saved_views/9',
      body: view,
    },
    {
      name: 'deleteSavedView',
      call: () => deleteSavedView(5, 9, CSRF),
      method: 'DELETE',
      url: '/api/projects/5/saved_views/9',
    },
  ]);
});

describe('notifications API', () => {
  afterEach(() => vi.unstubAllGlobals());

  itSendsEachRequest([
    {
      name: 'getNotifications defaults to the latest 50',
      call: () => getNotifications(),
      method: 'GET',
      url: '/api/notifications?limit=50',
    },
    {
      name: 'getNotifications unread only',
      call: () => getNotifications(true, 10),
      method: 'GET',
      url: '/api/notifications?unread_only=true&limit=10',
    },
    {
      name: 'getUnreadCount unwraps the count',
      call: () => getUnreadCount(),
      method: 'GET',
      url: '/api/notifications/unread-count',
      response: { json: { count: 4 } },
      returns: 4,
    },
    {
      name: 'markNotificationRead',
      call: () => markNotificationRead(8, CSRF),
      method: 'PATCH',
      url: '/api/notifications/8/read',
    },
    {
      name: 'markAllNotificationsRead',
      call: () => markAllNotificationsRead(CSRF),
      method: 'POST',
      url: '/api/notifications/read-all',
    },
    {
      name: 'getNotificationPreferences',
      call: () => getNotificationPreferences(),
      method: 'GET',
      url: '/api/notifications/preferences',
    },
    {
      name: 'setNotificationPreference',
      call: () => setNotificationPreference(5, { notify_email: false }, CSRF),
      method: 'PUT',
      url: '/api/notifications/preferences/5',
      body: { notify_email: false },
    },
    {
      name: 'deleteNotificationPreference',
      call: () => deleteNotificationPreference(5, CSRF),
      method: 'DELETE',
      url: '/api/notifications/preferences/5',
    },
  ]);
});

describe('traceability API', () => {
  afterEach(() => vi.unstubAllGlobals());

  itSendsEachRequest([
    {
      name: 'clearTraceabilitySuspect',
      call: () => clearTraceabilitySuspect(42, 77, CSRF),
      method: 'POST',
      url: '/api/traceability/clear_suspect',
      body: { req_id: 42, verification_id: 77 },
    },
    {
      name: 'getDsm with the default parameters',
      call: () => getDsm(5, DEFAULT_DSM_PARAMS),
      method: 'GET',
      url: '/api/projects/5/dsm',
    },
    {
      name: 'getDsm with a partition order and a subtree',
      call: () => getDsm(5, { ...DEFAULT_DSM_PARAMS, order: 'partition', categoryId: 2, rootId: 42 }),
      method: 'GET',
      url: '/api/projects/5/dsm?order=partition&category_id=2&root_id=42',
    },
  ]);
});

describe('getDashboard', () => {
  afterEach(() => vi.unstubAllGlobals());

  it('normalises the wire projects', async () => {
    stubFetch({
      json: {
        user: { id: 1, username: 'alice' },
        projects: [
          { project_id: 5, project_slug: 'space-project', project_name: 'Space Project' },
          {
            project_id: 6,
            project_slug: 'rover',
            project_base_path: '/payload/rover',
            group_id: 2,
            group_name: 'Payload',
            group_slug: 'payload',
          },
        ],
        projects_count: 2,
        selected_project_id: 5,
        selected_project_slug: 'space-project',
        csrf_token: 'csrf',
      },
    });

    const dashboard = await getDashboard();

    expect(dashboard.projects[0]).toMatchObject({
      id: 5,
      slug: 'space-project',
      project_base_path: '/space-project',
      group_id: null,
      group_name: null,
      group_slug: null,
    });
    expect(dashboard.projects[1]).toMatchObject({
      id: 6,
      slug: 'rover',
      project_base_path: '/payload/rover',
      group_id: 2,
      group_name: 'Payload',
      group_slug: 'payload',
    });
    expect(dashboard).toMatchObject({ projects_count: 2, selected_project_id: 5, csrf_token: 'csrf' });
  });
});

// Issue #285: the notification poll must not keep an idle session alive.
describe('notification polling', () => {
  afterEach(() => vi.unstubAllGlobals());

  it('marks the unread-count request as background', async () => {
    const fetchMock = stubFetch({ json: { count: 1 } });
    await getUnreadCount();
    expect(lastRequest(fetchMock).headers['X-Marreq-Background']).toBe('1');
  });
});
