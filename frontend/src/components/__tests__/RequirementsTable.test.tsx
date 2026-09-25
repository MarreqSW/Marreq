import { useState } from 'react';
import { MemoryRouter, useSearchParams } from 'react-router-dom';
import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { Requirement } from '@/api/types';
import RequirementsTable from '../RequirementsTable';

vi.mock('@/context/DashboardContext', () => ({
  useDashboard: () => ({
    csrfToken: 'csrf-test',
    dashboard: { user: { id: 7 } },
  }),
}));

vi.mock('@/api/client', () => ({
  getMyPermissions: vi.fn(),
  getSavedView: vi.fn(),
  listCategories: vi.fn(),
  listProjectMembers: vi.fn(),
  listRequirementStatuses: vi.fn(),
  listRequirements: vi.fn(),
  listSavedViews: vi.fn(),
  listUsersOptional: vi.fn(),
  listVerificationMethodsByProject: vi.fn(),
  patchRequirementByProject: vi.fn(),
  downloadRequirementsXlsx: vi.fn(),
  createSavedView: vi.fn(),
  updateSavedView: vi.fn(),
  deleteSavedView: vi.fn(),
}));

import * as api from '@/api/client';

const alpha: Requirement = {
  id: 1,
  current_version_id: 11,
  title: 'Authentication timeout',
  description: 'The system shall terminate inactive authenticated sessions after 30 minutes.',
  status_id: 2,
  author_id: 7,
  reviewer_id: 8,
  reference_code: 'REQ-SYS-042',
  category_id: 3,
  parent_id: 9,
  parent_requirement_ids: [9],
  creation_date: '2026-01-01T00:00:00Z',
  update_date: '2026-09-20T12:00:00Z',
  deadline_date: null,
  applicability_id: 1,
  justification: null,
  project_id: 5,
  approval_state: 'approved',
  approved_by: 7,
  approved_at: '2026-09-21T12:00:00Z',
  verification_method_ids: [4, 5],
};

const parent: Requirement = {
  ...alpha,
  id: 9,
  current_version_id: 19,
  title: 'Session security',
  description: 'Parent statement',
  reference_code: 'REQ-SYS-010',
  parent_id: null,
  parent_requirement_ids: [],
  approval_state: 'draft',
  verification_method_ids: [],
};

function Harness({ search = '', globalSearch = '' }: { search?: string; globalSearch?: string }) {
  return (
    <MemoryRouter initialEntries={[`/demo/requirements${search}`]}>
      <RequirementsHarness globalSearch={globalSearch} />
    </MemoryRouter>
  );
}

function RequirementsHarness({ globalSearch: initialSearch }: { globalSearch: string }) {
  const [searchParams] = useSearchParams();
  const [globalSearch, setGlobalSearch] = useState(initialSearch);
  return (
    <RequirementsTable
      projectId={5}
      basePath="/demo"
      globalSearch={globalSearch}
      setGlobalSearch={setGlobalSearch}
      viewMode={searchParams.get('view') === 'list' ? 'list' : 'table'}
    />
  );
}

function StatefulModeHarness() {
  const [searchParams, setSearchParams] = useSearchParams();
  const [globalSearch, setGlobalSearch] = useState('');
  const viewMode = searchParams.get('view') === 'list' ? 'list' : 'table';
  return (
    <>
      <button type="button" onClick={() => setSearchParams({ view: 'list' })}>
        Show List
      </button>
      <button type="button" onClick={() => setSearchParams({})}>
        Show Table
      </button>
      <RequirementsTable
        projectId={5}
        basePath="/demo"
        globalSearch={globalSearch}
        setGlobalSearch={setGlobalSearch}
        viewMode={viewMode}
      />
    </>
  );
}

describe('RequirementsTable views', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(api.listRequirements).mockResolvedValue([alpha, parent]);
    vi.mocked(api.listRequirementStatuses).mockResolvedValue([
      {
        id: 2,
        title: 'Ready for review',
        description: '',
        tag: 'review',
        project_id: 5,
        is_system: false,
        tag_color: '#22c55e',
      },
    ]);
    vi.mocked(api.listUsersOptional).mockResolvedValue(null);
    vi.mocked(api.listCategories).mockResolvedValue([
      { id: 3, title: 'Security', description: '', tag: 'SEC', project_id: 5 },
    ]);
    vi.mocked(api.listProjectMembers).mockResolvedValue([
      { user_id: 7, role: 1, role_label: 'Editor', username: 'alice', name: 'Alice' },
    ]);
    vi.mocked(api.listVerificationMethodsByProject).mockResolvedValue([
      { id: 4, title: 'Test', description: '', tag: 'TEST', project_id: 5 },
      { id: 5, title: 'Analysis', description: '', tag: 'ANALYSIS', project_id: 5 },
    ]);
    vi.mocked(api.getMyPermissions).mockResolvedValue({
      view_requirements: true,
      edit_requirements: true,
      approve_versions: true,
      manage_custom_fields: false,
      manage_project_members: false,
      is_project_reviewer: true,
    });
    vi.mocked(api.listSavedViews).mockResolvedValue([]);
  });

  it('defaults to the management table and does not patch an unchanged inline edit', async () => {
    const user = userEvent.setup();
    render(<Harness />);

    expect(await screen.findByRole('table')).toBeInTheDocument();
    expect(screen.getByText('Columns')).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: alpha.title }));
    const input = screen.getByDisplayValue(alpha.title);
    await user.click(input);
    await user.tab();
    expect(api.patchRequirementByProject).not.toHaveBeenCalled();
  });

  it('renders List as a contextual review surface with the statement and core metadata', async () => {
    render(<Harness search="?view=list" />);

    expect(await screen.findByRole('list', { name: 'Requirements for review' })).toBeInTheDocument();
    expect(screen.queryByRole('table')).not.toBeInTheDocument();
    expect(screen.queryByText('Columns')).not.toBeInTheDocument();
    const article = screen.getByRole('heading', { name: alpha.title }).closest('article');
    expect(article).not.toBeNull();
    expect(article!.querySelector('header')).toHaveClass('flex-col', 'sm:flex-row');
    expect(article!.querySelector('dl')).toHaveClass('grid-cols-1', 'sm:grid-cols-2');
    const card = within(article!);
    expect(card.getByText(alpha.description)).toBeInTheDocument();
    expect(card.getByText('REQ-SYS-042')).toBeInTheDocument();
    expect(card.getByText('Ready for review')).toBeInTheDocument();
    expect(card.getByText('APPROVED')).toBeInTheDocument();
    expect(card.getByText('Security')).toBeInTheDocument();
    expect(card.getByText('Test, Analysis')).toBeInTheDocument();
    expect(card.getByText('Alice (alice)')).toBeInTheDocument();
    expect(card.getByRole('link', { name: 'REQ-SYS-010' })).toBeInTheDocument();
    expect(card.getByRole('link', { name: /Duplicate REQ-SYS-042/i })).toBeInTheDocument();
  });

  it('keeps permission-dependent List actions hidden from viewers', async () => {
    vi.mocked(api.getMyPermissions).mockResolvedValue({
      view_requirements: true,
      edit_requirements: false,
      approve_versions: false,
      manage_custom_fields: false,
      manage_project_members: false,
      is_project_reviewer: false,
    });
    render(<Harness search="?view=list" />);

    await screen.findByText(alpha.description);
    expect(screen.queryByRole('link', { name: /Duplicate/i })).not.toBeInTheDocument();
    expect(screen.getAllByRole('link', { name: 'View' })).toHaveLength(2);
    expect(screen.getAllByRole('link', { name: 'Edit' })).toHaveLength(2);
  });

  it('applies search and preserves the List empty-result behavior', async () => {
    render(<Harness search="?view=list" globalSearch="does-not-exist" />);

    expect(await screen.findByText('No requirements match filters.')).toBeInTheDocument();
    expect(screen.queryByText(alpha.description)).not.toBeInTheDocument();
  });

  it('filters List results with the shared category filter', async () => {
    const user = userEvent.setup();
    vi.mocked(api.listCategories).mockResolvedValue([
      { id: 3, title: 'Security', description: '', tag: 'SEC', project_id: 5 },
      { id: 8, title: 'Platform', description: '', tag: 'PLAT', project_id: 5 },
    ]);
    vi.mocked(api.listRequirements).mockResolvedValue([
      alpha,
      { ...parent, category_id: 8 },
    ]);
    render(<Harness search="?view=list" />);

    await screen.findByText(alpha.description);
    await user.selectOptions(screen.getByLabelText('Filter by category'), '3');
    expect(screen.getByText(alpha.description)).toBeInTheDocument();
    await waitFor(() => expect(screen.queryByText(parent.description)).not.toBeInTheDocument());
  });

  it('preserves Table column preferences across List round trips', async () => {
    const user = userEvent.setup();
    render(
      <MemoryRouter initialEntries={['/demo/requirements']}>
        <StatefulModeHarness />
      </MemoryRouter>,
    );

    await screen.findByRole('table');
    await user.click(screen.getByText('Columns'));
    await user.click(screen.getByRole('checkbox', { name: 'Modified' }));
    expect(screen.queryByRole('columnheader', { name: 'Modified' })).not.toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: 'Show List' }));
    expect(await screen.findByRole('list', { name: 'Requirements for review' })).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Show Table' }));
    expect(await screen.findByRole('table')).toBeInTheDocument();
    expect(screen.queryByRole('columnheader', { name: 'Modified' })).not.toBeInTheDocument();
  });

  it('paginates the same result set in List', async () => {
    const user = userEvent.setup();
    const requirements = Array.from({ length: 26 }, (_, index) => ({
      ...alpha,
      id: index + 1,
      current_version_id: index + 101,
      reference_code: `REQ-${String(index + 1).padStart(3, '0')}`,
      title: `Requirement ${index + 1}`,
      description: `Statement ${index + 1}`,
      parent_id: null,
      parent_requirement_ids: [],
    }));
    vi.mocked(api.listRequirements).mockResolvedValue(requirements);
    render(<Harness search="?view=list" />);

    await screen.findByText('Statement 1');
    expect(screen.queryByText('Statement 26')).not.toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: '2' }));
    expect(await screen.findByText('Statement 26')).toBeInTheDocument();
    expect(screen.queryByText('Statement 1')).not.toBeInTheDocument();
  });

  it('restores List mode from a saved view without losing its table columns', async () => {
    vi.mocked(api.getSavedView).mockResolvedValue({
      id: 12,
      project_id: 5,
      owner_id: 7,
      name: 'Review queue',
      description: null,
      visibility: 'private',
      definition: {
        version: 1,
        entity: 'requirements',
        filters: {},
        sort: { column: 'title', dir: 'asc' },
        columns: ['key', 'title'],
        ui: { view_mode: 'list', page_size: 50 },
      },
      locked: false,
      locked_at: null,
      created_at: '2026-01-01T00:00:00Z',
      updated_at: '2026-01-01T00:00:00Z',
    });
    render(<Harness search="?saved_view=12" />);

    expect(await screen.findByText(alpha.description)).toBeInTheDocument();
    expect(screen.queryByText('Columns')).not.toBeInTheDocument();
    expect(api.getSavedView).toHaveBeenCalledWith(5, 12);
  });
});
