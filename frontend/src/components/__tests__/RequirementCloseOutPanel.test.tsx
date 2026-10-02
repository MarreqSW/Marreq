import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as apiClient from '@/api/client';
import type { RequirementCloseOut } from '@/api/types';
import RequirementCloseOutPanel from '../RequirementCloseOutPanel';

vi.mock('@/api/client');

function closeOut(patch: Partial<RequirementCloseOut> = {}): RequirementCloseOut {
  return {
    requirement_id: 1,
    compliance: null,
    note: null,
    set_by: null,
    set_at: null,
    close_out: { status: 'open', reason: 'Compliance not assessed' },
    verifications: [],
    ...patch,
  };
}

describe('RequirementCloseOutPanel', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(apiClient.getRequirementCloseOut).mockResolvedValue(closeOut());
  });

  it('lets a reviewer assess compliance and shows the new close-out', async () => {
    vi.mocked(apiClient.putRequirementCompliance).mockResolvedValue(
      closeOut({
        compliance: 'PC',
        note: 'RFW-3',
        close_out: { status: 'closed', reason: 'Accepted (PC)' },
      }),
    );
    render(<RequirementCloseOutPanel projectId={5} requirementId={1} canAssess csrfToken="csrf" />);

    expect(await screen.findByText('Compliance not assessed')).toBeInTheDocument();
    expect(screen.getByText('Open')).toBeInTheDocument();
    const save = screen.getByRole('button', { name: 'Save assessment' });
    expect(save).toBeDisabled();

    await userEvent.selectOptions(screen.getByLabelText('Compliance'), 'PC');
    await userEvent.type(screen.getByLabelText('Note'), ' RFW-3 ');
    await userEvent.click(save);

    expect(apiClient.putRequirementCompliance).toHaveBeenCalledWith(5, 1, 'PC', 'RFW-3', 'csrf');
    expect(await screen.findByText('Accepted (PC)')).toBeInTheDocument();
    expect(screen.getByText('Closed')).toBeInTheDocument();
    expect(screen.getByText('Saved')).toBeInTheDocument();
  });

  it('clears the assessment with "Not assessed"', async () => {
    vi.mocked(apiClient.getRequirementCloseOut).mockResolvedValue(
      closeOut({ compliance: 'C', close_out: { status: 'closed', reason: 'Accepted (C)' } }),
    );
    vi.mocked(apiClient.putRequirementCompliance).mockResolvedValue(closeOut());
    render(<RequirementCloseOutPanel projectId={5} requirementId={1} canAssess csrfToken="csrf" />);
    await screen.findByText('Accepted (C)');
    await userEvent.selectOptions(screen.getByLabelText('Compliance'), '');
    await userEvent.click(screen.getByRole('button', { name: 'Save assessment' }));
    expect(apiClient.putRequirementCompliance).toHaveBeenCalledWith(5, 1, null, null, 'csrf');
  });

  it('is read-only for non-reviewers', async () => {
    render(<RequirementCloseOutPanel projectId={5} requirementId={1} canAssess={false} csrfToken="csrf" />);
    await screen.findByText('Compliance not assessed');
    expect(screen.getByLabelText('Compliance')).toBeDisabled();
    expect(screen.queryByRole('button', { name: 'Save assessment' })).not.toBeInTheDocument();
    expect(screen.getByText('Only project reviewers can assess compliance.')).toBeInTheDocument();
  });

  it('shows a save error', async () => {
    vi.mocked(apiClient.putRequirementCompliance).mockRejectedValue(new Error('Access denied.'));
    render(<RequirementCloseOutPanel projectId={5} requirementId={1} canAssess csrfToken="csrf" />);
    await screen.findByText('Compliance not assessed');
    await userEvent.selectOptions(screen.getByLabelText('Compliance'), 'NC');
    await userEvent.click(screen.getByRole('button', { name: 'Save assessment' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Access denied.');
  });
});
