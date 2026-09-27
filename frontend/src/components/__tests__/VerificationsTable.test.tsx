import { MemoryRouter, Outlet, Route, Routes, useLocation, useSearchParams } from 'react-router-dom';
import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { EffectivePermissions, Verification, VerificationStatus } from '@/api/types';
import VerificationsTable from '../VerificationsTable';
import VerificationsViewSwitcher from '../VerificationsViewSwitcher';

vi.mock('@/context/DashboardContext', () => ({
  useDashboard: () => ({ csrfToken: 'csrf-test', dashboard: { user: { id: 7 } } }),
}));

vi.mock('@/api/client', () => ({
  getMyPermissions: vi.fn(),
  listProjectMembers: vi.fn(),
  listUsersOptional: vi.fn(),
  listVerificationMethodsByProject: vi.fn(),
  listVerificationStatuses: vi.fn(),
  listVerificationsByProject: vi.fn(),
  updateVerificationField: vi.fn(),
  downloadVerificationsXlsx: vi.fn(),
}));

import * as api from '@/api/client';

const status = (id: number, title: string): VerificationStatus => ({
  id,
  title,
  description: '',
  tag: title.toLowerCase(),
  project_id: 5,
  is_system: false,
  tag_color: '#3366cc',
});

const ver = (id: number, over: Partial<Verification> = {}): Verification => ({
  id,
  name: `Check ${id}`,
  reference_code: `VER-${String(id).padStart(3, '0')}`,
  description: '',
  source: 'Lab',
  status_id: 1,
  parent_id: null,
  project_id: 5,
  verification_method_id: 4,
  author_id: 7,
  reviewer_id: 7,
  ...over,
});

const reviewerPerms = {
  view_requirements: true,
  edit_requirements: true,
  approve_versions: true,
  is_project_reviewer: true,
} as EffectivePermissions;

function Location() {
  const loc = useLocation();
  return <div data-testid="location">{loc.pathname + loc.search}</div>;
}

function TableRoute() {
  const [sp] = useSearchParams();
  return (
    <>
      <VerificationsViewSwitcher />
      <VerificationsTable
        projectId={5}
        basePath="/demo"
        globalSearch={sp.get('q') ?? ''}
        viewMode={sp.get('view') === 'list' ? 'list' : 'table'}
      />
      <Location />
    </>
  );
}

function renderTable(search = '') {
  return render(
    <MemoryRouter initialEntries={[`/demo/verifications${search}`]}>
      <Routes>
        <Route
          element={<Outlet context={{ projectId: 5, basePath: '/demo', globalSearch: '', setGlobalSearch: vi.fn() }} />}
        >
          <Route path="/demo/verifications" element={<TableRoute />} />
        </Route>
      </Routes>
    </MemoryRouter>,
  );
}

function tableRefs() {
  return screen.queryAllByText(/^VER-\d{3}$/).map((el) => el.textContent);
}

describe('VerificationsTable', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(api.listVerificationsByProject).mockResolvedValue([
      ver(1, { status_id: 1 }),
      ver(2, { status_id: 2, name: 'Thermal vacuum' }),
      ver(3, { status_id: 2, verification_method_id: null }),
      ver(22, { status_id: 1, name: 'Battery cycling' }),
    ]);
    vi.mocked(api.listVerificationStatuses).mockResolvedValue([
      status(1, 'Pending'),
      status(2, 'Passed'),
      status(3, 'Failed'),
    ]);
    vi.mocked(api.listVerificationMethodsByProject).mockResolvedValue([
      { id: 4, title: 'Test', description: '', tag: 'T', project_id: 5 },
    ]);
    vi.mocked(api.getMyPermissions).mockResolvedValue(reviewerPerms);
    vi.mocked(api.listUsersOptional).mockResolvedValue([]);
    vi.mocked(api.listProjectMembers).mockResolvedValue([]);
    vi.mocked(api.updateVerificationField).mockResolvedValue(undefined as never);
  });

  it('loads project verifications and shows project-wide status metrics', async () => {
    renderTable();
    await screen.findByText('VER-022');
    expect(api.listVerificationsByProject).toHaveBeenCalledWith(5);
    expect(screen.getByTestId('metric-total')).toHaveTextContent('4');
    expect(screen.getByTestId('metric-pass-rate')).toHaveTextContent('50%');
    expect(screen.getByRole('button', { name: /Pending\s*2/ })).toBeInTheDocument();
    expect(screen.queryByText('Category')).not.toBeInTheDocument();
  });

  it('filters by a metric chip, keeps metrics project-wide, and clears on second click', async () => {
    renderTable();
    await screen.findByText('VER-022');
    const user = userEvent.setup();

    await user.click(screen.getByRole('button', { name: /Passed\s*2/ }));
    expect(tableRefs()).toEqual(['VER-002', 'VER-003']);
    expect(screen.getByTestId('location')).toHaveTextContent('status=2');
    expect(screen.getByTestId('metric-total')).toHaveTextContent('4');

    await user.click(screen.getByRole('button', { name: /Passed\s*2/ }));
    expect(tableRefs()).toHaveLength(4);
    expect(screen.getByTestId('location')).not.toHaveTextContent('status=');
  });

  it('filters by method, including verifications without a method', async () => {
    renderTable();
    await screen.findByText('VER-022');
    const user = userEvent.setup();
    const method = screen.getByRole('combobox', { name: 'Filter by verification method' });

    await user.selectOptions(method, 'none');
    expect(tableRefs()).toEqual(['VER-003']);
    await user.selectOptions(method, '4');
    expect(tableRefs()).toEqual(['VER-001', 'VER-002', 'VER-022']);

    await user.click(screen.getByRole('button', { name: /Reset Filters/ }));
    expect(tableRefs()).toHaveLength(4);
  });

  it('keeps filters when switching between Table and List', async () => {
    renderTable('?status=1&method=4');
    await screen.findByText('VER-022');
    expect(tableRefs()).toEqual(['VER-001', 'VER-022']);
    const user = userEvent.setup();

    await user.click(screen.getByRole('link', { name: /List/ }));
    expect(screen.getByTestId('location')).toHaveTextContent('status=1&method=4&view=list');
    expect(screen.getByRole('link', { name: /List/ })).toHaveAttribute('aria-current', 'page');
    expect(tableRefs()).toEqual(['VER-001', 'VER-022']);

    await user.click(screen.getByRole('link', { name: /Table/ }));
    expect(screen.getByTestId('location')).toHaveTextContent('/demo/verifications?status=1&method=4');
  });

  it('shows the search empty state', async () => {
    renderTable('?q=nothing-matches');
    expect(await screen.findAllByText('No verifications match filters.')).not.toHaveLength(0);
  });

  it('shows pagination above and below when there is more than one page', async () => {
    vi.mocked(api.listVerificationsByProject).mockResolvedValue(
      Array.from({ length: 30 }, (_, i) => ver(i + 1)),
    );
    renderTable();
    await screen.findByText('VER-001');
    const top = screen.getByTestId('pagination-top');
    expect(top).toHaveTextContent('Showing 1-25 of 30');
    expect(screen.getAllByText(/^Showing/)).toHaveLength(2);

    const user = userEvent.setup();
    await user.click(within(top).getByRole('button', { name: '2' }));
    expect(screen.getByTestId('pagination-top')).toHaveTextContent('Showing 26-30 of 30');
    expect(screen.getByText('VER-030')).toBeInTheDocument();
  });

  it('lets a project reviewer change a status inline', async () => {
    renderTable();
    await screen.findByText('VER-022');
    const user = userEvent.setup();
    const row = screen.getByText('VER-022').closest('tr') as HTMLElement;

    await user.click(within(row).getByTitle('Click to edit status'));
    await user.selectOptions(within(row).getByRole('combobox'), '2');

    await waitFor(() =>
      expect(api.updateVerificationField).toHaveBeenCalledWith(5, 22, 'status_id', '2', 'csrf-test'),
    );
    expect(screen.getByRole('button', { name: /Passed\s*3/ })).toBeInTheDocument();
  });

  it('does not offer inline status editing to non-reviewers', async () => {
    vi.mocked(api.getMyPermissions).mockResolvedValue({ ...reviewerPerms, is_project_reviewer: false });
    renderTable();
    await screen.findByText('VER-022');
    expect(screen.queryByTitle('Click to edit status')).not.toBeInTheDocument();
  });

  it('labels the exports', async () => {
    renderTable();
    await screen.findByText('VER-022');
    expect(screen.getByRole('button', { name: 'Download CSV (filtered rows)' })).toBeInTheDocument();
    expect(screen.getByTitle('Download Excel (whole project)')).toBeInTheDocument();
  });
});
