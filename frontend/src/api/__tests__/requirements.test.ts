import { afterEach, describe, expect, it, vi } from 'vitest';
import { compareRequirementVersionsByProject } from '../requirements';

describe('compareRequirementVersionsByProject', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('uses the project-scoped requirement version diff endpoint', async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      text: async () => JSON.stringify({
        text: {
          title: { added: [], removed: [], unchanged: ['Title'] },
          description: { added: [], removed: [], unchanged: ['Statement'] },
          justification: { added: [], removed: [], unchanged: [] },
        },
        metadata: {
          status: { unchanged: 1 },
          category: { unchanged: 2 },
          applicability: { unchanged: 3 },
          verification: { added_ids: [], removed_ids: [], unchanged_ids: [] },
          custom_fields: [],
        },
      }),
    });
    vi.stubGlobal('fetch', fetchMock);

    await compareRequirementVersionsByProject(7, 42, 101, 102);

    expect(fetchMock).toHaveBeenCalledWith(
      '/api/projects/7/requirements/42/versions/101/diff/102',
      expect.objectContaining({ credentials: 'same-origin' }),
    );
  });
});
