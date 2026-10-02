import { afterEach, describe, expect, it, vi } from 'vitest';
import { CSRF, itSendsEachRequest, stubFetch } from '@/test/apiHarness';
import {
  createProject,
  deleteProject,
  getCoverageReport,
  getMyPermissions,
  getProjectFromPath,
  getProjectReviewers,
  listProjectMembers,
  listProjectsOptional,
  putProjectReviewers,
  removeProjectMember,
  setProjectMemberRole,
  updateProject,
} from '../projects';

describe('projects API', () => {
  afterEach(() => vi.unstubAllGlobals());

  itSendsEachRequest([
    {
      name: 'createProject',
      call: () => createProject({ name: 'Lunar Rover', group_id: 2 }, CSRF),
      method: 'POST',
      url: '/api/projects',
      body: { name: 'Lunar Rover', group_id: 2 },
      response: { json: { id: 6, slug: 'lunar-rover' } },
      returns: { id: 6, slug: 'lunar-rover' },
    },
    {
      name: 'updateProject sends only the given fields',
      call: () => updateProject(5, { status: 'OnHold' }, CSRF),
      method: 'PATCH',
      url: '/api/projects/5',
      body: { status: 'OnHold' },
    },
    {
      name: 'deleteProject confirms with the slug and accepts 204',
      call: () => deleteProject(5, 'space-project', CSRF),
      method: 'DELETE',
      url: '/api/projects/5',
      body: { confirm_slug: 'space-project' },
      response: { status: 204 },
    },
    {
      name: 'getProjectFromPath encodes the slug',
      call: () => getProjectFromPath('a b/c'),
      method: 'GET',
      url: '/api/project-from-path/a%20b%2Fc',
    },
    {
      name: 'listProjectMembers',
      call: () => listProjectMembers(5),
      method: 'GET',
      url: '/api/projects/5/members',
    },
    {
      name: 'getProjectReviewers',
      call: () => getProjectReviewers(5),
      method: 'GET',
      url: '/api/projects/5/reviewers',
    },
    {
      name: 'putProjectReviewers',
      call: () => putProjectReviewers(5, [1, 2], CSRF),
      method: 'PUT',
      url: '/api/projects/5/reviewers',
      body: { user_ids: [1, 2] },
    },
    {
      name: 'setProjectMemberRole',
      call: () => setProjectMemberRole(5, 2, 3, CSRF),
      method: 'PUT',
      url: '/api/projects/5/members/2',
      body: { role: 3 },
    },
    {
      name: 'removeProjectMember',
      call: () => removeProjectMember(5, 2, CSRF),
      method: 'DELETE',
      url: '/api/projects/5/members/2',
    },
    {
      name: 'getCoverageReport',
      call: () => getCoverageReport(5),
      method: 'GET',
      url: '/api/projects/5/coverage_report',
    },
    {
      name: 'getMyPermissions',
      call: () => getMyPermissions(5),
      method: 'GET',
      url: '/api/projects/5/me/permissions',
    },
    {
      name: 'listProjectsOptional returns the list',
      call: () => listProjectsOptional(),
      method: 'GET',
      url: '/api/projects',
      response: { json: [{ id: 5 }] },
      returns: [{ id: 5 }],
    },
  ]);

  it('listProjectsOptional returns null when the request fails', async () => {
    stubFetch({ status: 403, json: { message: 'forbidden' } });
    await expect(listProjectsOptional()).resolves.toBeNull();
  });

  it('deleteProject surfaces the slug mismatch message', async () => {
    stubFetch({ status: 400, json: { message: 'confirm_slug does not match the project' } });
    await expect(deleteProject(5, 'wrong', CSRF)).rejects.toMatchObject({
      status: 400,
      message: 'confirm_slug does not match the project',
    });
  });
});
