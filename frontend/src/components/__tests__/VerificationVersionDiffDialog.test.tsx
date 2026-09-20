import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import VerificationVersionDiffDialog from '../VerificationVersionDiffDialog';
import type { VerificationSnapshot, VerificationVersionDiff } from '@/api/types';
import { compareVerificationSnapshotsByProject } from '@/api/client';

vi.mock('@/api/client', () => ({
  compareVerificationSnapshotsByProject: vi.fn(),
}));

const snapshots: VerificationSnapshot[] = [
  {
    id: 30,
    created_at: '2026-01-03T00:00:00Z',
    name: 'Third',
    reference_code: 'VER-001',
    status_id: 2,
  },
  {
    id: 10,
    created_at: '2026-01-01T00:00:00Z',
    name: 'First',
    reference_code: 'VER-001',
    status_id: 1,
  },
  {
    id: 20,
    created_at: '2026-01-02T00:00:00Z',
    name: 'Second',
    reference_code: 'VER-001',
    status_id: 1,
  },
];

const diff: VerificationVersionDiff = {
  text: {
    name: { removed: ['Second'], added: ['Third'], unchanged: [] },
    description: {
      removed: ['Measure 500W'],
      added: ['Measure 650W'],
      unchanged: [],
    },
    source: { removed: [], added: [], unchanged: ['TV-001'] },
    reference_code: { removed: [], added: [], unchanged: ['VER-001'] },
  },
  metadata: {
    status: {
      old_id: 1,
      new_id: 2,
      old_label: 'Not run',
      new_label: 'Passed',
    },
    verification_method: { unchanged: 1, unchanged_label: 'Test' },
    parent: {},
  },
};

describe('VerificationVersionDiffDialog', () => {
  beforeEach(() => {
    vi.mocked(compareVerificationSnapshotsByProject).mockReset();
  });

  it('compares the latest snapshot with its predecessor by default', async () => {
    vi.mocked(compareVerificationSnapshotsByProject).mockResolvedValue(diff);
    render(
      <VerificationVersionDiffDialog
        open
        onClose={vi.fn()}
        projectId={2}
        verificationId={4}
        snapshots={snapshots}
      />,
    );

    await waitFor(() =>
      expect(compareVerificationSnapshotsByProject).toHaveBeenCalledWith(2, 4, 20, 30),
    );
    expect(await screen.findByText('Measure 650W')).toBeInTheDocument();
    expect(screen.getByText('Measure 500W')).toBeInTheDocument();
  });

  it('loads a selected snapshot pair', async () => {
    vi.mocked(compareVerificationSnapshotsByProject).mockResolvedValue(diff);
    const user = userEvent.setup();
    render(
      <VerificationVersionDiffDialog
        open
        onClose={vi.fn()}
        projectId={2}
        verificationId={4}
        snapshots={snapshots}
      />,
    );

    await waitFor(() => expect(compareVerificationSnapshotsByProject).toHaveBeenCalledTimes(1));
    await user.selectOptions(screen.getByLabelText(/older version/i), '10');
    await waitFor(() =>
      expect(compareVerificationSnapshotsByProject).toHaveBeenLastCalledWith(2, 4, 10, 30),
    );
  });

  it('closes on Escape', async () => {
    vi.mocked(compareVerificationSnapshotsByProject).mockResolvedValue(diff);
    const onClose = vi.fn();
    const user = userEvent.setup();
    render(
      <VerificationVersionDiffDialog
        open
        onClose={onClose}
        projectId={2}
        verificationId={4}
        snapshots={snapshots}
      />,
    );
    await user.keyboard('{Escape}');
    expect(onClose).toHaveBeenCalled();
  });

  it('shows API errors', async () => {
    vi.mocked(compareVerificationSnapshotsByProject).mockRejectedValue(
      new Error('Diff unavailable'),
    );
    render(
      <VerificationVersionDiffDialog
        open
        onClose={vi.fn()}
        projectId={2}
        verificationId={4}
        snapshots={snapshots}
      />,
    );

    expect(await screen.findByText('Diff unavailable')).toBeInTheDocument();
  });

  it('requires at least two saved versions', () => {
    render(
      <VerificationVersionDiffDialog
        open
        onClose={vi.fn()}
        projectId={2}
        verificationId={4}
        snapshots={[snapshots[0]]}
      />,
    );

    expect(screen.getByText(/at least two saved versions/i)).toBeInTheDocument();
    expect(compareVerificationSnapshotsByProject).not.toHaveBeenCalled();
  });
});
