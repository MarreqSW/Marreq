import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ThemeProvider } from '@/context/ThemeContext';
import BackupPage from '../BackupPage';
import type { ProjectOutletContext } from '@/types/projectOutlet';

const mocks = vi.hoisted(() => ({
  listUsersOptional: vi.fn(),
  getDeploymentInfo: vi.fn(),
  getCsrfToken: vi.fn(),
  downloadDatabaseBackup: vi.fn(),
  getBackupInfo: vi.fn(),
}));

vi.mock('@/api/client', () => mocks);

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
  return { ...actual, useOutletContext: vi.fn() };
});

import { useOutletContext } from 'react-router-dom';

const serverDeployment = {
  mode: 'server',
  allows_self_registration: false,
  requires_email_verification: false,
  allows_admin_promotion: true,
  assigns_personal_workspace: false,
  allows_self_administered_user_creation: true,
  allows_database_backup: true,
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
      <MemoryRouter initialEntries={['/space-project/admin/backup']}>
        <Routes>
          <Route path="/:projectSlug/admin/backup" element={<BackupPage />} />
        </Routes>
      </MemoryRouter>
    </ThemeProvider>,
  );
}

describe('BackupPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.listUsersOptional.mockResolvedValue([{ id: 1, username: 'alice' }]);
    mocks.getDeploymentInfo.mockResolvedValue(serverDeployment);
    mocks.getBackupInfo.mockResolvedValue({
      attachments_available: true,
      attachment_files: 12,
      attachment_bytes: 3 * 1024 * 1024,
    });
  });

  it('shows access denied for non-admins', async () => {
    mocks.listUsersOptional.mockResolvedValue(null);
    renderPage();
    expect(await screen.findByText('Access denied')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Download backup' })).not.toBeInTheDocument();
  });

  it('downloads a backup with the CSRF token and reports the filename', async () => {
    let finish: (name: string) => void = () => {};
    mocks.downloadDatabaseBackup.mockReturnValue(
      new Promise<string>((resolve) => {
        finish = resolve;
      }),
    );
    renderPage();
    const user = userEvent.setup();
    await user.click(await screen.findByRole('button', { name: 'Download backup' }));

    expect(mocks.downloadDatabaseBackup).toHaveBeenCalledWith('csrf-token', { attachments: true });
    expect(screen.getByRole('button', { name: 'Generating backup…' })).toBeDisabled();

    finish('marreq-backup_20260926_101500.tar.gz');
    expect(
      await screen.findByText('Downloaded marreq-backup_20260926_101500.tar.gz.'),
    ).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Download backup' })).toBeEnabled();
  });

  // Issue #341: files are included by default, with their size, and can be left out.
  it('includes the attachment files unless unticked', async () => {
    mocks.downloadDatabaseBackup.mockResolvedValue('marreq-backup.tar.gz');
    renderPage();
    const user = userEvent.setup();
    const box = await screen.findByRole('checkbox', { name: /Include attachment files \(12 files, 3(\.0)? MB\)/ });
    expect(box).toBeChecked();

    await user.click(box);
    await user.click(screen.getByRole('button', { name: 'Download backup' }));
    expect(mocks.downloadDatabaseBackup).toHaveBeenCalledWith('csrf-token', { attachments: false });
  });

  it('explains a database-only backup when the server has no attachment storage', async () => {
    mocks.getBackupInfo.mockResolvedValue({
      attachments_available: false,
      attachment_files: 0,
      attachment_bytes: 0,
    });
    mocks.downloadDatabaseBackup.mockResolvedValue('marreq-backup.tar.gz');
    renderPage();
    expect(await screen.findByTestId('backup-no-attachments')).toBeInTheDocument();
    expect(screen.queryByRole('checkbox')).not.toBeInTheDocument();
    await userEvent.setup().click(screen.getByRole('button', { name: 'Download backup' }));
    expect(mocks.downloadDatabaseBackup).toHaveBeenCalledWith('csrf-token', { attachments: false });
  });

  it('shows backend errors', async () => {
    mocks.downloadDatabaseBackup.mockRejectedValue(new Error('pg_dump failed: could not connect'));
    renderPage();
    const user = userEvent.setup();
    await user.click(await screen.findByRole('button', { name: 'Download backup' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('pg_dump failed: could not connect');
  });

  it('hides the download when backups are disabled for the deployment', async () => {
    mocks.getDeploymentInfo.mockResolvedValue({ ...serverDeployment, allows_database_backup: false });
    renderPage();
    expect(await screen.findByTestId('backup-disabled')).toBeInTheDocument();
    await waitFor(() =>
      expect(screen.queryByRole('button', { name: 'Download backup' })).not.toBeInTheDocument(),
    );
  });
});
