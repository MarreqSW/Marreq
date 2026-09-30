import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as apiClient from '@/api/client';
import type { ProjectStorage } from '@/api/types';
import ProjectStorageSettings from '../ProjectStorageSettings';

vi.mock('@/api/client');

const MB = 1024 * 1024;
const storage: ProjectStorage = {
  used_bytes: 50 * MB,
  quota_bytes: 500 * MB,
  quota_is_default: true,
  default_quota_bytes: 500 * MB,
  retained_by_baselines_bytes: 5 * MB,
  max_file_bytes: 10 * MB,
  allowed_extensions: ['pdf', 'png'],
};

beforeEach(() => {
  vi.mocked(apiClient.getProjectStorage).mockReset().mockResolvedValue(storage);
  vi.mocked(apiClient.setProjectStorageQuota).mockReset();
});

describe('ProjectStorageSettings', () => {
  it('shows usage and limits read-only for non-admins', async () => {
    render(<ProjectStorageSettings projectId={5} isAdmin={false} csrfToken="csrf" />);
    expect(await screen.findByText(/50 MB of 500 MB used \(10%\)/)).toBeInTheDocument();
    expect(screen.getByText(/5 MB of it are deleted files that baselines still keep/)).toBeInTheDocument();
    expect(screen.getByText(/at most 10 MB each\. Allowed types: pdf, png\./)).toBeInTheDocument();
    expect(screen.getByText('Only instance administrators can change the quota.')).toBeInTheDocument();
    expect(screen.queryByLabelText('Quota (MB)')).not.toBeInTheDocument();
  });

  it('lets instance admins set and reset the quota', async () => {
    vi.mocked(apiClient.setProjectStorageQuota)
      .mockResolvedValueOnce({ ...storage, quota_bytes: 2048 * MB, quota_is_default: false })
      .mockResolvedValueOnce(storage);
    const user = userEvent.setup();
    render(<ProjectStorageSettings projectId={5} isAdmin csrfToken="csrf" />);
    const input = await screen.findByLabelText('Quota (MB)');
    expect(input).toHaveValue(500);
    await user.clear(input);
    await user.type(input, '2048');
    await user.click(screen.getByRole('button', { name: 'Set quota' }));
    expect(apiClient.setProjectStorageQuota).toHaveBeenCalledWith(5, 2048, 'csrf');
    expect(await screen.findByText(/set for this project/)).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: 'Reset to default' }));
    expect(apiClient.setProjectStorageQuota).toHaveBeenLastCalledWith(5, null, 'csrf');
    expect(await screen.findByText(/instance default/)).toBeInTheDocument();
  });

  it('validates the quota before sending', async () => {
    const user = userEvent.setup();
    render(<ProjectStorageSettings projectId={5} isAdmin csrfToken="csrf" />);
    const input = await screen.findByLabelText('Quota (MB)');
    await user.clear(input);
    await user.type(input, '0');
    await user.click(screen.getByRole('button', { name: 'Set quota' }));
    expect(screen.getByRole('alert')).toHaveTextContent('at least 1');
    expect(apiClient.setProjectStorageQuota).not.toHaveBeenCalled();
  });
});
