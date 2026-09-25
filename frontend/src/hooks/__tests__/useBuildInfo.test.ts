import { describe, expect, it, vi, beforeEach } from 'vitest';
import * as apiClient from '@/api/client';
import { loadBuildInfo, resetBuildInfoCache } from '../useBuildInfo';

vi.mock('@/api/client');

const build = {
  backend_version: '0.1.3',
  backend_git_sha: 'abc',
  deployment_mode: 'server',
  frontend_compatibility: { min_version: '0.1.0', max_version: '0.1.99' },
};

describe('loadBuildInfo', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    resetBuildInfoCache();
  });

  it('shares one request between callers', async () => {
    vi.mocked(apiClient.getBuildInfo).mockResolvedValue(build);
    const [a, b] = await Promise.all([loadBuildInfo(), loadBuildInfo()]);
    expect(a).toEqual(build);
    expect(b).toBe(a);
    expect(apiClient.getBuildInfo).toHaveBeenCalledTimes(1);
  });

  it('retries after a failure', async () => {
    vi.mocked(apiClient.getBuildInfo)
      .mockRejectedValueOnce(new Error('offline'))
      .mockResolvedValueOnce(build);
    await expect(loadBuildInfo()).rejects.toThrow('offline');
    await expect(loadBuildInfo()).resolves.toEqual(build);
    expect(apiClient.getBuildInfo).toHaveBeenCalledTimes(2);
  });
});
