import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { Verification, VerificationStatus } from '@/api/types';
import VerificationStatusMetrics, { computeStatusMetrics } from '../VerificationStatusMetrics';

const status = (id: number, title: string): VerificationStatus => ({
  id,
  title,
  description: '',
  tag: title.toLowerCase(),
  project_id: 5,
  is_system: false,
  tag_color: '#22aa55',
});

const ver = (id: number, status_id: number): Verification => ({
  id,
  name: `Test ${id}`,
  reference_code: `VER-${id}`,
  description: '',
  source: '',
  status_id,
  parent_id: null,
  project_id: 5,
  verification_method_id: null,
  author_id: 1,
  reviewer_id: 1,
});

const statuses = [status(1, 'Pending'), status(2, 'Passed'), status(3, 'Failed')];
const rows = [ver(1, 1), ver(2, 2), ver(3, 2), ver(4, 99)];

describe('computeStatusMetrics', () => {
  it('counts per status, keeps unknown statuses in Other, and finds Passed', () => {
    const m = computeStatusMetrics(rows, statuses);
    expect(m.total).toBe(4);
    expect(m.byStatus.map((s) => s.count)).toEqual([1, 2, 0]);
    expect(m.other).toBe(1);
    expect(m.passed).toBe(2);
  });

  it('has no pass count without a Passed status', () => {
    expect(computeStatusMetrics(rows, [status(1, 'Pending')]).passed).toBeNull();
  });
});

describe('VerificationStatusMetrics', () => {
  it('shows totals, pass rate, zero statuses, and Other', () => {
    render(
      <VerificationStatusMetrics
        verifications={rows}
        statuses={statuses}
        activeStatusId={null}
        onSelectStatus={vi.fn()}
      />,
    );
    expect(screen.getByTestId('metric-total')).toHaveTextContent('4');
    expect(screen.getByTestId('metric-pass-rate')).toHaveTextContent('50%');
    expect(screen.getByText('2 of 4 passed')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Failed\s*0/ })).toBeInTheDocument();
    expect(screen.getByTestId('status-chip-other')).toHaveTextContent('Other1');
  });

  it('hides the pass rate when there is no Passed status', () => {
    render(
      <VerificationStatusMetrics
        verifications={rows}
        statuses={[status(1, 'Pending')]}
        activeStatusId={null}
        onSelectStatus={vi.fn()}
      />,
    );
    expect(screen.queryByTestId('metric-pass-rate')).not.toBeInTheDocument();
  });

  it('toggles the status filter from a chip', async () => {
    const onSelect = vi.fn();
    const user = userEvent.setup();
    const { rerender } = render(
      <VerificationStatusMetrics
        verifications={rows}
        statuses={statuses}
        activeStatusId={null}
        onSelectStatus={onSelect}
      />,
    );
    const passed = screen.getByRole('button', { name: /Passed\s*2/ });
    expect(passed).toHaveAttribute('aria-pressed', 'false');
    await user.click(passed);
    expect(onSelect).toHaveBeenLastCalledWith(2);

    rerender(
      <VerificationStatusMetrics
        verifications={rows}
        statuses={statuses}
        activeStatusId={2}
        onSelectStatus={onSelect}
      />,
    );
    expect(screen.getByRole('button', { name: /Passed\s*2/ })).toHaveAttribute('aria-pressed', 'true');
    await user.click(screen.getByRole('button', { name: /Passed\s*2/ }));
    expect(onSelect).toHaveBeenLastCalledWith(null);
  });
});
