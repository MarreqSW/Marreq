import { afterEach, describe, expect, it, vi } from 'vitest';
import { CSRF, itSendsEachRequest, stubFetch } from '@/test/apiHarness';
import {
  createRequirementByProject,
  createRequirementComment,
  createRequirementVersionLink,
  deleteRequirementGlobally,
  deleteRequirementVersionLink,
  getRequirementByProject,
  listRequirementActivityByProject,
  listRequirementComments,
  listRequirementVersionLinks,
  listRequirementVersionLinkTypes,
  listRequirements,
  listRequirementVersionsByProject,
  patchRequirementByProject,
  setRequirementVersionApproval,
} from '../requirements';

const link = { source_version_id: 2, target_version_id: 1, link_type: 'DERIVES_FROM', rationale: null };

describe('requirements API', () => {
  afterEach(() => vi.unstubAllGlobals());

  itSendsEachRequest([
    {
      name: 'listRequirements without filters',
      call: () => listRequirements(5),
      method: 'GET',
      url: '/api/projects/5/requirements',
      response: { json: [] },
    },
    {
      name: 'listRequirements encodes every filter',
      call: () =>
        listRequirements(5, {
          approval_state: 'approved',
          has_tests: false,
          status_id: 1,
          category_id: 2,
          q: 'power & thermal',
          sort_column: 'title',
          sort_dir: 'desc',
          view_id: 7,
        }),
      method: 'GET',
      url:
        '/api/projects/5/requirements?approval_state=approved&has_tests=false&status_id=1&category_id=2' +
        '&q=power+%26+thermal&sort_column=title&sort_dir=desc&view_id=7',
      response: { json: [] },
    },
    {
      name: 'createRequirementByProject returns the new id',
      call: () => createRequirementByProject(5, { title: 'Power' } as never, CSRF),
      method: 'POST',
      url: '/api/projects/5/requirements',
      body: { title: 'Power' },
      response: { json: { status: 'created', id: 42 } },
      returns: { id: 42 },
    },
    {
      name: 'getRequirementByProject',
      call: () => getRequirementByProject(5, 42),
      method: 'GET',
      url: '/api/projects/5/requirements/42',
    },
    {
      name: 'patchRequirementByProject drops undefined fields',
      call: () => patchRequirementByProject(5, 42, { title: 'New', description: undefined } as never, CSRF),
      method: 'PATCH',
      url: '/api/projects/5/requirements/42',
      body: { title: 'New' },
    },
    {
      name: 'deleteRequirementGlobally',
      call: () => deleteRequirementGlobally(42, CSRF),
      method: 'DELETE',
      url: '/api/requirements/42',
    },
    {
      name: 'setRequirementVersionApproval',
      call: () => setRequirementVersionApproval(5, 42, 100, 'approved', CSRF),
      method: 'PUT',
      url: '/api/projects/5/requirements/42/versions/100/approval',
      body: { state: 'approved' },
    },
    {
      name: 'listRequirementVersionsByProject',
      call: () => listRequirementVersionsByProject(5, 42),
      method: 'GET',
      url: '/api/projects/5/requirements/42/versions',
    },
    {
      name: 'listRequirementActivityByProject',
      call: () => listRequirementActivityByProject(5, 42),
      method: 'GET',
      url: '/api/projects/5/requirements/42/activity',
    },
    {
      name: 'listRequirementComments for all versions',
      call: () => listRequirementComments(42),
      method: 'GET',
      url: '/api/requirements/42/comments',
    },
    {
      name: 'listRequirementComments for one version',
      call: () => listRequirementComments(42, 100),
      method: 'GET',
      url: '/api/requirements/42/comments?version_id=100',
    },
    {
      name: 'listRequirementComments ignores a NaN version',
      call: () => listRequirementComments(42, Number.NaN),
      method: 'GET',
      url: '/api/requirements/42/comments',
    },
    {
      name: 'createRequirementComment',
      call: () => createRequirementComment(42, { body: 'Looks good', requirement_version_id: 100 }, CSRF),
      method: 'POST',
      url: '/api/requirements/42/comments',
      body: { body: 'Looks good', requirement_version_id: 100 },
    },
    {
      name: 'listRequirementVersionLinks without filters',
      call: () => listRequirementVersionLinks(5),
      method: 'GET',
      url: '/api/projects/5/requirement-version-links',
    },
    {
      name: 'listRequirementVersionLinks with filters',
      call: () => listRequirementVersionLinks(5, { source_version_id: 2, target_version_id: 1, link_type: 'REFINES' }),
      method: 'GET',
      url: '/api/projects/5/requirement-version-links?source_version_id=2&target_version_id=1&link_type=REFINES',
    },
    {
      name: 'createRequirementVersionLink',
      call: () => createRequirementVersionLink(5, link, CSRF),
      method: 'POST',
      url: '/api/projects/5/requirement-version-links',
      body: link,
    },
    {
      name: 'deleteRequirementVersionLink',
      call: () => deleteRequirementVersionLink(5, 8, CSRF),
      method: 'DELETE',
      url: '/api/projects/5/requirement-version-links/8',
    },
    {
      name: 'listRequirementVersionLinkTypes unwraps link_types',
      call: () => listRequirementVersionLinkTypes(5),
      method: 'GET',
      url: '/api/projects/5/requirement-version-links/link-types',
      response: { json: { link_types: ['DERIVES_FROM', 'REFINES'] } },
      returns: ['DERIVES_FROM', 'REFINES'],
    },
    {
      name: 'listRequirementVersionLinkTypes tolerates a missing list',
      call: () => listRequirementVersionLinkTypes(5),
      method: 'GET',
      url: '/api/projects/5/requirement-version-links/link-types',
      returns: [],
    },
  ]);

  it('patchRequirementByProject refuses an empty patch without calling the server', async () => {
    const fetchMock = stubFetch();
    await expect(patchRequirementByProject(5, 42, { title: undefined } as never, CSRF)).rejects.toThrow(
      'No changes to save',
    );
    expect(fetchMock).not.toHaveBeenCalled();
  });
});
