import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ThemeProvider } from '@/context/ThemeContext';
import * as apiClient from '@/api/client';
import * as tableUtils from '@/utils/tableUtils';
import RequirementsTable from '../RequirementsTable';
import type { Requirement } from '@/api/types';

vi.mock('@/api/client');

vi.mock('@/context/DashboardContext', () => ({
  useDashboard: () => ({ csrfToken: 'csrf-token' }),
}));

const perms = {
  view_requirements: true,
  edit_requirements: true,
  approve_versions: false,
  is_project_reviewer: true,
  manage_custom_fields: false,
  manage_project_members: false,
};

const status = {
  id: 1,
  title: 'Draft',
  description: '',
  tag: 'draft',
  project_id: 5,
  is_system: true,
  tag_color: null,
};

const req = (overrides: Partial<Requirement> = {}): Requirement => ({
  id: 101,
  current_version_id: 1,
  title: 'Alpha requirement',
  description: 'desc',
  status_id: 1,
  author_id: 1,
  reviewer_id: 1,
  reference_code: 'REQ-101',
  category_id: 1,
  parent_id: null,
  creation_date: '2024-01-01T00:00:00',
  update_date: '2024-01-02T12:00:00',
  deadline_date: null,
  applicability_id: 1,
  justification: null,
  project_id: 5,
  approval_state: 'not_requested',
  approved_by: null,
  approved_at: null,
  verification_method_ids: [],
  ...overrides,
});

function mockLoadSuccess(rows: Requirement[] = [req()]) {
  vi.mocked(apiClient.listRequirements).mockResolvedValue(rows);
  vi.mocked(apiClient.listRequirementStatuses).mockResolvedValue([status]);
  vi.mocked(apiClient.listUsersOptional).mockResolvedValue([
    {
      id: 1,
      username: 'alice',
      name: 'Alice',
      email: 'a@b.c',
      creation_date: '',
      last_login: '',
      is_admin: false,
    },
  ]);
  vi.mocked(apiClient.listCategories).mockResolvedValue([
    { id: 1, title: 'Functional', description: '', tag: 'fn', project_id: 5 },
  ]);
  vi.mocked(apiClient.listProjectMembers).mockResolvedValue([
    { user_id: 1, username: 'alice', name: 'Alice', role: 1, role_label: 'member' },
  ]);
  vi.mocked(apiClient.listVerificationMethodsByProject).mockResolvedValue([]);
  vi.mocked(apiClient.getMyPermissions).mockResolvedValue(perms);
}

function renderTable(
  props?: Partial<{
    projectId: number;
    basePath: string;
    globalSearch: string;
    viewMode: 'table' | 'list';
  }>,
) {
  return render(
    <ThemeProvider>
      <MemoryRouter>
        <RequirementsTable
          projectId={5}
          basePath="/alice/space"
          globalSearch=""
          viewMode="table"
          {...props}
        />
      </MemoryRouter>
    </ThemeProvider>,
  );
}

describe('RequirementsTable', () => {
  beforeEach(() => {
    vi.resetAllMocks();
  });

  it('loads and shows requirement rows', async () => {
    mockLoadSuccess();
    renderTable();
    expect(await screen.findByText('Alpha requirement')).toBeInTheDocument();
    expect(screen.getByText(/1 Requirements found/i)).toBeInTheDocument();
    expect(screen.getByText('REQ-101')).toBeInTheDocument();
  });

  it('shows error state when load fails', async () => {
    vi.mocked(apiClient.listRequirements).mockRejectedValue(new Error('boom'));
    vi.mocked(apiClient.listRequirementStatuses).mockResolvedValue([]);
    vi.mocked(apiClient.listUsersOptional).mockResolvedValue([]);
    vi.mocked(apiClient.listCategories).mockResolvedValue([]);
    vi.mocked(apiClient.listProjectMembers).mockResolvedValue([]);
    vi.mocked(apiClient.listVerificationMethodsByProject).mockResolvedValue([]);
    vi.mocked(apiClient.getMyPermissions).mockResolvedValue(perms);
    renderTable();
    expect(await screen.findByText('boom')).toBeInTheDocument();
  });

  it('filters by globalSearch and shows empty match message', async () => {
    mockLoadSuccess([req(), req({ id: 102, title: 'Beta item', reference_code: 'REQ-102' })]);
    renderTable({ globalSearch: 'zzz-no-match' });
    expect(await screen.findByText(/No requirements match filters/i)).toBeInTheDocument();
  });

  it('renders list view mode', async () => {
    mockLoadSuccess();
    renderTable({ viewMode: 'list' });
    expect(await screen.findByText('Alpha requirement')).toBeInTheDocument();
    expect(screen.getByRole('list')).toBeInTheDocument();
  });

  it('exports CSV via downloadCsv', async () => {
    mockLoadSuccess();
    const spy = vi.spyOn(tableUtils, 'downloadCsv').mockImplementation(() => {});
    renderTable();
    await screen.findByText('Alpha requirement');
    await userEvent.click(screen.getByTitle('Download CSV'));
    expect(spy).toHaveBeenCalledWith(
      'requirements-project-5.csv',
      expect.any(Array),
      expect.any(Array),
    );
    spy.mockRestore();
  });

  it('inline-edits title and patches when changed', async () => {
    mockLoadSuccess();
    vi.mocked(apiClient.patchRequirementByProject).mockResolvedValue(undefined);
    renderTable();
    const title = await screen.findByText('Alpha requirement');
    await userEvent.click(title);
    const input = screen.getByDisplayValue('Alpha requirement');
    await userEvent.clear(input);
    await userEvent.type(input, 'Updated title');
    await userEvent.tab();
    await waitFor(() =>
      expect(apiClient.patchRequirementByProject).toHaveBeenCalledWith(
        5,
        101,
        { title: 'Updated title' },
        'csrf-token',
      ),
    );
  });

  it('does not allow inline edit without edit permission', async () => {
    mockLoadSuccess();
    vi.mocked(apiClient.getMyPermissions).mockResolvedValue({
      ...perms,
      edit_requirements: false,
    });
    renderTable();
    const title = await screen.findByText('Alpha requirement');
    expect(title.tagName.toLowerCase()).not.toBe('button');
    await userEvent.click(title);
    expect(screen.queryByDisplayValue('Alpha requirement')).not.toBeInTheDocument();
  });

  it('filters by status select', async () => {
    mockLoadSuccess([
      req(),
      req({ id: 102, title: 'Other', reference_code: 'REQ-102', status_id: 2 }),
    ]);
    vi.mocked(apiClient.listRequirementStatuses).mockResolvedValue([
      status,
      { ...status, id: 2, title: 'Approved', tag: 'ok' },
    ]);
    renderTable();
    await screen.findByText('Alpha requirement');
    const statusSelect = screen.getByDisplayValue('All');
    await userEvent.selectOptions(statusSelect, '1');
    expect(screen.getByText('Alpha requirement')).toBeInTheDocument();
    expect(screen.queryByText('Other')).not.toBeInTheDocument();
  });
});
