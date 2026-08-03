import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ThemeProvider } from '@/context/ThemeContext';
import * as apiClient from '@/api/client';
import * as tableUtils from '@/utils/tableUtils';
import VerificationsTable from '../VerificationsTable';
import type { Verification } from '@/api/types';

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
  title: 'Open',
  description: '',
  tag: 'open',
  project_id: 5,
  is_system: true,
  tag_color: null,
};

const ver = (overrides: Partial<Verification> = {}): Verification => ({
  id: 201,
  name: 'Verify alpha',
  reference_code: 'VER-201',
  description: 'd',
  source: 'lab',
  status_id: 1,
  parent_id: null,
  project_id: 5,
  verification_method_id: null,
  author_id: 1,
  reviewer_id: 1,
  ...overrides,
});

function mockLoadSuccess(rows: Verification[] = [ver()]) {
  vi.mocked(apiClient.listVerifications).mockResolvedValue(rows);
  vi.mocked(apiClient.listVerificationStatuses).mockResolvedValue([status]);
  vi.mocked(apiClient.listVerificationMethodsByProject).mockResolvedValue([]);
  vi.mocked(apiClient.getMyPermissions).mockResolvedValue(perms);
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
        <VerificationsTable
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

describe('VerificationsTable', () => {
  beforeEach(() => {
    vi.resetAllMocks();
  });

  it('loads and shows verification rows for the project', async () => {
    mockLoadSuccess([ver(), ver({ id: 99, name: 'Other project', project_id: 9 })]);
    renderTable();
    expect(await screen.findByText('Verify alpha')).toBeInTheDocument();
    expect(screen.queryByText('Other project')).not.toBeInTheDocument();
    expect(screen.getByText(/1 Verifications found|1 verification/i)).toBeTruthy();
  });

  it('shows error state when load fails', async () => {
    vi.mocked(apiClient.listVerifications).mockRejectedValue(new Error('load failed'));
    vi.mocked(apiClient.listVerificationStatuses).mockResolvedValue([]);
    vi.mocked(apiClient.listVerificationMethodsByProject).mockResolvedValue([]);
    vi.mocked(apiClient.getMyPermissions).mockResolvedValue(perms);
    vi.mocked(apiClient.listUsersOptional).mockResolvedValue([]);
    renderTable();
    expect(await screen.findByText('load failed')).toBeInTheDocument();
  });

  it('shows empty message when globalSearch matches nothing', async () => {
    mockLoadSuccess();
    renderTable({ globalSearch: 'nope' });
    expect(await screen.findByText(/No verifications match filters/i)).toBeInTheDocument();
  });

  it('renders list view mode', async () => {
    mockLoadSuccess();
    renderTable({ viewMode: 'list' });
    expect(await screen.findByText('Verify alpha')).toBeInTheDocument();
  });

  it('exports CSV via downloadCsv', async () => {
    mockLoadSuccess();
    const spy = vi.spyOn(tableUtils, 'downloadCsv').mockImplementation(() => {});
    renderTable();
    await screen.findByText('Verify alpha');
    await userEvent.click(screen.getByTitle('Download CSV'));
    expect(spy).toHaveBeenCalled();
    spy.mockRestore();
  });

  it('inline-edits name via updateVerificationField', async () => {
    mockLoadSuccess();
    vi.mocked(apiClient.updateVerificationField).mockResolvedValue(undefined);
    renderTable();
    const name = await screen.findByText('Verify alpha');
    await userEvent.click(name);
    const input = screen.getByDisplayValue('Verify alpha');
    await userEvent.clear(input);
    await userEvent.type(input, 'Renamed');
    await userEvent.tab();
    await waitFor(() =>
      expect(apiClient.updateVerificationField).toHaveBeenCalledWith(
        5,
        201,
        'name',
        'Renamed',
        'csrf-token',
      ),
    );
  });
});
