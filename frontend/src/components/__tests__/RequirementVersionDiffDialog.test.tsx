import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import RequirementVersionDiffDialog from '../RequirementVersionDiffDialog';
import type { RequirementDiff, RequirementVersion } from '@/api/types';
import { compareRequirementVersionsByProject } from '@/api/client';

vi.mock('@/api/client', () => ({
  compareRequirementVersionsByProject: vi.fn(),
}));

const versions: RequirementVersion[] = [
  {
    id: 30,
    requirement_id: 4,
    title: 'Third',
    description: 'Third statement',
    status_id: 2,
    author_id: 1,
    reviewer_id: 2,
    category_id: 3,
    applicability_id: 4,
    justification: 'Third rationale',
    deadline_date: null,
    created_at: '2026-01-03T00:00:00Z',
    approval_state: 'draft',
    approved_by: null,
    approved_at: null,
  },
  {
    id: 10,
    requirement_id: 4,
    title: 'First',
    description: 'First statement',
    status_id: 1,
    author_id: 1,
    reviewer_id: 2,
    category_id: 3,
    applicability_id: 4,
    justification: null,
    deadline_date: null,
    created_at: '2026-01-01T00:00:00Z',
    approval_state: 'draft',
    approved_by: null,
    approved_at: null,
  },
  {
    id: 20,
    requirement_id: 4,
    title: 'Second',
    description: 'Second statement',
    status_id: 1,
    author_id: 1,
    reviewer_id: 2,
    category_id: 3,
    applicability_id: 4,
    justification: 'Second rationale',
    deadline_date: null,
    created_at: '2026-01-02T00:00:00Z',
    approval_state: 'reviewed',
    approved_by: null,
    approved_at: null,
  },
];

const diff: RequirementDiff = {
  text: {
    title: { removed: ['Second'], added: ['Third'], unchanged: [] },
    description: {
      removed: ['Old power statement'],
      added: ['New power statement'],
      unchanged: ['Shared line'],
    },
    justification: { removed: [], added: ['Third rationale'], unchanged: [] },
  },
  metadata: {
    status: {
      old_id: 1,
      new_id: 2,
      old_label: 'Draft',
      new_label: 'Approved',
    },
    category: { old_id: 3, new_id: 3, unchanged: 3, unchanged_label: 'Power' },
    applicability: { old_id: 4, new_id: 4, unchanged: 4, unchanged_label: 'All missions' },
    verification: {
      added_ids: [9],
      removed_ids: [],
      unchanged_ids: [8],
      added_labels: ['Test'],
      unchanged_labels: ['Analysis'],
    },
    custom_fields: [
      {
        field_id: 11,
        label: 'Criticality',
        old_value: 'Low',
        new_value: 'High',
        unchanged: false,
      },
    ],
  },
};

describe('RequirementVersionDiffDialog', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(compareRequirementVersionsByProject).mockResolvedValue(diff);
  });

  it('defaults to the latest version compared with its predecessor and renders all sections', async () => {
    render(
      <RequirementVersionDiffDialog
        open
        onClose={vi.fn()}
        projectId={2}
        requirementId={4}
        versions={versions}
      />,
    );

    await waitFor(() =>
      expect(compareRequirementVersionsByProject).toHaveBeenCalledWith(2, 4, 20, 30),
    );
    expect(await screen.findByText('Old power statement')).toBeInTheDocument();
    expect(screen.getByText('New power statement')).toBeInTheDocument();
    expect(screen.getByText('Rationale')).toBeInTheDocument();
    expect(screen.getByText('Third rationale')).toBeInTheDocument();
    expect(screen.getByText('Draft')).toBeInTheDocument();
    expect(screen.getByText('Approved')).toBeInTheDocument();
    expect(screen.getByText('Criticality')).toBeInTheDocument();
    expect(screen.getByText('High')).toBeInTheDocument();
  });

  it('loads a newly selected valid version pair', async () => {
    const user = userEvent.setup();
    render(
      <RequirementVersionDiffDialog
        open
        onClose={vi.fn()}
        projectId={2}
        requirementId={4}
        versions={versions}
      />,
    );
    await waitFor(() => expect(compareRequirementVersionsByProject).toHaveBeenCalledTimes(1));

    await user.selectOptions(screen.getByLabelText('Older version'), '10');

    await waitFor(() =>
      expect(compareRequirementVersionsByProject).toHaveBeenLastCalledWith(2, 4, 10, 30),
    );
  });

  it('closes on Escape', async () => {
    const onClose = vi.fn();
    const user = userEvent.setup();
    render(
      <RequirementVersionDiffDialog
        open
        onClose={onClose}
        projectId={2}
        requirementId={4}
        versions={versions}
      />,
    );

    await user.keyboard('{Escape}');

    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('shows an error returned by the comparison API', async () => {
    vi.mocked(compareRequirementVersionsByProject).mockRejectedValue(new Error('Diff unavailable'));
    render(
      <RequirementVersionDiffDialog
        open
        onClose={vi.fn()}
        projectId={2}
        requirementId={4}
        versions={versions}
      />,
    );

    expect(await screen.findByText('Diff unavailable')).toBeInTheDocument();
  });

  it('requires at least two saved versions', () => {
    render(
      <RequirementVersionDiffDialog
        open
        onClose={vi.fn()}
        projectId={2}
        requirementId={4}
        versions={[versions[0]]}
      />,
    );

    expect(screen.getByText(/at least two saved versions/i)).toBeInTheDocument();
    expect(compareRequirementVersionsByProject).not.toHaveBeenCalled();
  });
});
