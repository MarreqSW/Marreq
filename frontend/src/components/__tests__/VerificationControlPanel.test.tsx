import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as apiClient from '@/api/client';
import type { VerificationControl } from '@/api/types';
import VerificationControlPanel from '../VerificationControlPanel';

vi.mock('@/api/client');

const empty: VerificationControl = {
  verification_id: 20,
  verification_level: null,
  verification_stage: 'QUAL',
  evidence_reference: null,
  updated_by: null,
  updated_at: null,
  suggestions: { levels: ['Subsystem', 'System'], stages: ['ACC', 'QUAL'] },
};

describe('VerificationControlPanel', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(apiClient.getVerificationControl).mockResolvedValue(empty);
  });

  it('saves trimmed level, stage and evidence', async () => {
    vi.mocked(apiClient.putVerificationControl).mockResolvedValue({
      ...empty,
      verification_level: 'Subsystem',
      evidence_reference: 'TR-PWR-002',
    });
    const { container } = render(
      <VerificationControlPanel projectId={5} verificationId={20} canEdit csrfToken="csrf" />,
    );
    const level = await screen.findByPlaceholderText('e.g. Subsystem');
    expect(screen.getByPlaceholderText('e.g. QUAL')).toHaveValue('QUAL');
    expect([...container.querySelectorAll('datalist option')].map((o) => o.getAttribute('value'))).toEqual([
      'Subsystem',
      'System',
      'ACC',
      'QUAL',
    ]);
    const save = screen.getByRole('button', { name: 'Save' });
    expect(save).toBeDisabled();

    await userEvent.type(level, ' Subsystem ');
    await userEvent.type(screen.getByPlaceholderText('e.g. TR-PWR-002'), 'TR-PWR-002');
    await userEvent.click(save);

    expect(apiClient.putVerificationControl).toHaveBeenCalledWith(
      5,
      20,
      { verification_level: 'Subsystem', verification_stage: 'QUAL', evidence_reference: 'TR-PWR-002' },
      'csrf',
    );
    expect(await screen.findByText('Saved')).toBeInTheDocument();
    expect(save).toBeDisabled();
  });

  it('is read-only without edit rights', async () => {
    vi.mocked(apiClient.getVerificationControl).mockResolvedValue({ ...empty, evidence_reference: 'TR-1' });
    render(<VerificationControlPanel projectId={5} verificationId={20} canEdit={false} csrfToken="csrf" />);
    expect(await screen.findByDisplayValue('TR-1')).toBeDisabled();
    expect(screen.queryByRole('button', { name: 'Save' })).not.toBeInTheDocument();
  });

  it('shows load and save errors', async () => {
    vi.mocked(apiClient.getVerificationControl).mockRejectedValueOnce(new Error('verification not found'));
    const { unmount } = render(
      <VerificationControlPanel projectId={5} verificationId={20} canEdit csrfToken="csrf" />,
    );
    expect(await screen.findByRole('alert')).toHaveTextContent('verification not found');
    unmount();

    vi.mocked(apiClient.putVerificationControl).mockRejectedValue(
      new Error('verification_stage must be at most 40 characters'),
    );
    render(<VerificationControlPanel projectId={5} verificationId={20} canEdit csrfToken="csrf" />);
    await userEvent.type(await screen.findByPlaceholderText('e.g. Subsystem'), 'System');
    await userEvent.click(screen.getByRole('button', { name: 'Save' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('at most 40 characters');
  });
});
