import { afterEach, describe, expect, it, vi } from 'vitest';
import { getDashboard } from '../dashboard';
import { mockFetchOk } from './fetchTestUtils';

describe('getDashboard', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it('maps wire projects to dashboard projects with defaults', async () => {
    vi.stubGlobal(
      'fetch',
      mockFetchOk({
        user: { id: 1, username: 'alice', is_admin: false },
        csrf_token: 'tok',
        projects: [
          {
            project_id: 10,
            project_slug: 'alpha',
            name: 'Alpha',
            description: null,
          },
          {
            project_id: 11,
            project_slug: 'beta',
            name: 'Beta',
            description: 'd',
            project_base_path: '/g/beta',
            group_id: 2,
            group_name: 'G',
            group_slug: 'g',
          },
        ],
      }),
    );

    const dash = await getDashboard();
    expect(dash.projects[0]).toMatchObject({
      id: 10,
      slug: 'alpha',
      project_base_path: '/alpha',
      group_id: null,
      group_name: null,
      group_slug: null,
    });
    expect(dash.projects[1]).toMatchObject({
      id: 11,
      project_base_path: '/g/beta',
      group_id: 2,
      group_name: 'G',
      group_slug: 'g',
    });
  });
});
