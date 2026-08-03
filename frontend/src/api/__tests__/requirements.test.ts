import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  createRequirementByProject,
  deleteRequirementGlobally,
  listRequirementComments,
  listRequirementVersionLinkTypes,
  listRequirementVersionLinks,
  patchRequirementByProject,
} from '../requirements';
import { lastFetchCall, mockFetchOk } from './fetchTestUtils';

describe('requirements api', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it('patchRequirementByProject strips undefined and throws when empty', async () => {
    await expect(
      patchRequirementByProject(1, 2, { title: undefined }, 'csrf'),
    ).rejects.toThrow('No changes to save');

    vi.stubGlobal('fetch', mockFetchOk());
    await patchRequirementByProject(1, 2, { title: 'T', description: undefined }, 'csrf');
    const [url, init] = lastFetchCall();
    expect(url).toBe('/api/projects/1/requirements/2');
    expect(init?.method).toBe('PATCH');
    expect(JSON.parse(String(init?.body))).toEqual({ title: 'T' });
    expect(init?.headers).toMatchObject({
      'Content-Type': 'application/json',
      'X-CSRF-Token': 'csrf',
    });
  });

  it('listRequirementComments adds version_id when finite', async () => {
    vi.stubGlobal('fetch', mockFetchOk([]));
    await listRequirementComments(9);
    expect(lastFetchCall()[0]).toBe('/api/requirements/9/comments');
    await listRequirementComments(9, 3);
    expect(lastFetchCall()[0]).toBe('/api/requirements/9/comments?version_id=3');
  });

  it('listRequirementVersionLinks builds query params', async () => {
    vi.stubGlobal('fetch', mockFetchOk([]));
    await listRequirementVersionLinks(1, {
      source_version_id: 2,
      target_version_id: 3,
      link_type: 'parent',
    });
    const url = lastFetchCall()[0];
    expect(url).toContain('/api/projects/1/requirement-version-links?');
    expect(url).toContain('source_version_id=2');
    expect(url).toContain('target_version_id=3');
    expect(url).toContain('link_type=parent');
  });

  it('listRequirementVersionLinkTypes defaults missing link_types to []', async () => {
    vi.stubGlobal('fetch', mockFetchOk({}));
    await expect(listRequirementVersionLinkTypes(1)).resolves.toEqual([]);
  });

  it('createRequirementByProject returns id; delete uses CSRF-only headers', async () => {
    vi.stubGlobal('fetch', mockFetchOk({ id: 42 }));
    await expect(
      createRequirementByProject(
        1,
        {
          title: 'T',
          description: 'd',
          author_id: 1,
          category_id: 1,
          status_id: 1,
          reference_code: 'R-1',
          reviewer_id: 1,
          applicability_id: 1,
          project_id: 1,
          verification_method_ids: [],
        },
        'csrf',
      ),
    ).resolves.toEqual({ id: 42 });

    vi.stubGlobal('fetch', mockFetchOk());
    await deleteRequirementGlobally(7, 'csrf');
    const [, init] = lastFetchCall();
    expect(init?.method).toBe('DELETE');
    expect(init?.headers).toEqual({ 'X-CSRF-Token': 'csrf' });
  });
});
