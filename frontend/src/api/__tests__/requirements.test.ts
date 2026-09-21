import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  compareBaselineRequirementWithCurrent,
  compareRequirementVersionsByProject,
  getRequirementVersionByProject,
} from '../requirements';

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

  it('uses the baseline requirement vs current endpoint', async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      text: async () => JSON.stringify({ text: {}, metadata: {} }),
    });
    vi.stubGlobal('fetch', fetchMock);

    await compareBaselineRequirementWithCurrent(7, 8, 42);

    expect(fetchMock).toHaveBeenCalledWith(
      '/api/projects/7/baselines/8/requirements/42/diff/current',
      expect.objectContaining({ credentials: 'same-origin' }),
    );
  });
});

describe('getRequirementVersionByProject', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('uses the project-scoped get version endpoint', async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      text: async () =>
        JSON.stringify({
          id: 101,
          requirement_id: 42,
          title: 'V1',
          description: 'Statement',
          status_id: 1,
          author_id: 1,
          reviewer_id: 2,
          category_id: 1,
          applicability_id: 1,
          justification: null,
          deadline_date: null,
          created_at: '2026-01-01T00:00:00Z',
          approval_state: 'approved',
          approved_by: 1,
          approved_at: '2026-01-02T00:00:00Z',
          custom_fields: [],
          verification_method_ids: [3],
        }),
    });
    vi.stubGlobal('fetch', fetchMock);

    await getRequirementVersionByProject(7, 42, 101);

    expect(fetchMock).toHaveBeenCalledWith(
      '/api/projects/7/requirements/42/versions/101',
      expect.objectContaining({ credentials: 'same-origin' }),
    );
  });
});
