import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  compareBaselineVerificationWithCurrent,
  compareVerificationSnapshotsByProject,
} from '../verifications';

describe('compareVerificationSnapshotsByProject', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('uses the project-scoped verification snapshot diff endpoint', async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      text: async () =>
        JSON.stringify({
          text: {
            name: { added: [], removed: [], unchanged: ['Power test'] },
            description: { added: [], removed: [], unchanged: ['Measure 650W'] },
            source: { added: [], removed: [], unchanged: ['TV-001'] },
            reference_code: { added: [], removed: [], unchanged: ['VER-001'] },
          },
          metadata: {
            status: { unchanged: 1 },
            verification_method: { unchanged: 2 },
            parent: {},
          },
        }),
    });
    vi.stubGlobal('fetch', fetchMock);

    await compareVerificationSnapshotsByProject(7, 42, 101, 102);

    expect(fetchMock).toHaveBeenCalledWith(
      '/api/projects/7/verifications/42/snapshots/101/diff/102',
      expect.objectContaining({ credentials: 'same-origin' }),
    );
  });

  it('uses the baseline verification vs current endpoint', async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      text: async () => JSON.stringify({ text: {}, metadata: {} }),
    });
    vi.stubGlobal('fetch', fetchMock);

    await compareBaselineVerificationWithCurrent(7, 8, 42);

    expect(fetchMock).toHaveBeenCalledWith(
      '/api/projects/7/baselines/8/verifications/42/diff/current',
      expect.objectContaining({ credentials: 'same-origin' }),
    );
  });
});
