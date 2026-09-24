import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ThemeProvider } from '@/context/ThemeContext';
import SystemLogsPage from '../SystemLogsPage';
import type { ProjectOutletContext } from '@/types/projectOutlet';

const mocks = vi.hoisted(() => ({
  listUsersOptional: vi.fn(),
  listAdminLogs: vi.fn(),
  downloadAdminLogsJson: vi.fn(),
  cleanupAdminLogs: vi.fn(),
  listRequirementStatuses: vi.fn(),
  listVerificationStatuses: vi.fn(),
  listCategories: vi.fn(),
  listApplicability: vi.fn(),
  listVerificationMethods: vi.fn(),
}));

vi.mock('@/api/client', () => ({
  listUsersOptional: mocks.listUsersOptional,
  listAdminLogs: mocks.listAdminLogs,
  downloadAdminLogsJson: mocks.downloadAdminLogsJson,
  cleanupAdminLogs: mocks.cleanupAdminLogs,
  listRequirementStatuses: mocks.listRequirementStatuses,
  listVerificationStatuses: mocks.listVerificationStatuses,
  listCategories: mocks.listCategories,
  listApplicability: mocks.listApplicability,
  listVerificationMethods: mocks.listVerificationMethods,
}));

vi.mock('@/context/DashboardContext', () => ({
  useDashboard: () => ({
    csrfToken: 'csrf-token',
    dashboard: {
      user: { id: 1, username: 'alice', name: 'Alice', is_admin: true },
      projects: [{ id: 5, name: 'Space Project' }],
    },
  }),
}));

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual<typeof import('react-router-dom')>('react-router-dom');
  return {
    ...actual,
    useOutletContext: vi.fn(),
  };
});

import { useOutletContext } from 'react-router-dom';

const sampleRow = {
  log_id: 9,
  user_id: 1,
  username: 'alice',
  action_type: 'CREATE',
  summary: 'Created requirement',
  description: null,
  created_at: '2024-06-01T12:00:00',
  changes: [
    { field: 'Status', old_value: '—', new_value: '13' },
    { field: 'Category', old_value: '—', new_value: '2' },
    { field: 'Applicability', old_value: '—', new_value: '8' },
  ],
  entity_type: 'REQUIREMENT',
  entity_id: 12,
  project_id: 5,
};

function renderPage() {
  vi.mocked(useOutletContext).mockReturnValue({
    projectId: 5,
    basePath: '/space-project',
    globalSearch: '',
    setGlobalSearch: vi.fn(),
  } satisfies ProjectOutletContext);

  return render(
    <ThemeProvider>
      <MemoryRouter initialEntries={['/space-project/admin/logs']}>
        <Routes>
          <Route path="/:projectSlug/admin/logs" element={<SystemLogsPage />} />
        </Routes>
      </MemoryRouter>
    </ThemeProvider>,
  );
}

describe('SystemLogsPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    window.confirm = vi.fn(() => true);
    mocks.listUsersOptional.mockResolvedValue([{ id: 1, username: 'alice' }]);
    mocks.listAdminLogs.mockResolvedValue({
      items: [sampleRow],
      total: 1,
      limit: 50,
      offset: 0,
    });
    mocks.downloadAdminLogsJson.mockResolvedValue(undefined);
    mocks.cleanupAdminLogs.mockResolvedValue({ deleted: 2 });
    mocks.listRequirementStatuses.mockResolvedValue([{ id: 13, title: 'Draft' }]);
    mocks.listVerificationStatuses.mockResolvedValue([]);
    mocks.listCategories.mockResolvedValue([{ id: 2, title: 'Functional' }]);
    mocks.listApplicability.mockResolvedValue([{ id: 8, title: 'Flight' }]);
    mocks.listVerificationMethods.mockResolvedValue([]);
  });

  it('shows access denied when admin APIs are forbidden', async () => {
    mocks.listUsersOptional.mockResolvedValue(null);
    renderPage();
    await waitFor(() => {
      expect(screen.getByText(/access denied/i)).toBeInTheDocument();
    });
    expect(mocks.listAdminLogs).not.toHaveBeenCalled();
  });

  it('applies filters to the list query', async () => {
    const user = userEvent.setup();
    renderPage();
    await waitFor(() => {
      expect(screen.getByText('Created requirement')).toBeInTheDocument();
    });

    await user.type(screen.getByText(/entity type/i).querySelector('input')!, 'REQUIREMENT');
    await user.click(screen.getByRole('button', { name: /^filter$/i }));

    await waitFor(() => {
      expect(mocks.listAdminLogs).toHaveBeenCalledWith(
        expect.objectContaining({
          entity_type: 'REQUIREMENT',
          limit: 50,
          offset: 0,
        }),
      );
    });
  });

  it('exports JSON and runs cleanup after confirm', async () => {
    const user = userEvent.setup();
    renderPage();
    await waitFor(() => {
      expect(screen.getByText('Created requirement')).toBeInTheDocument();
    });

    await user.click(screen.getByRole('button', { name: /export json/i }));
    await waitFor(() => {
      expect(mocks.downloadAdminLogsJson).toHaveBeenCalled();
    });

    await user.click(screen.getByRole('button', { name: /^cleanup$/i }));
    await waitFor(() => {
      expect(window.confirm).toHaveBeenCalled();
      expect(mocks.cleanupAdminLogs).toHaveBeenCalledWith(90, 'csrf-token');
    });
  });

  it('shows catalog titles instead of ids in expanded change details', async () => {
    const user = userEvent.setup();
    renderPage();
    await waitFor(() => {
      expect(screen.getByText('Created requirement')).toBeInTheDocument();
    });
    await user.click(screen.getByText('Created requirement'));
    expect(screen.getByText('Draft')).toBeInTheDocument();
    expect(screen.getByText('Functional')).toBeInTheDocument();
    expect(screen.getByText('Flight')).toBeInTheDocument();
    expect(screen.queryByText('13')).not.toBeInTheDocument();
  });
});
