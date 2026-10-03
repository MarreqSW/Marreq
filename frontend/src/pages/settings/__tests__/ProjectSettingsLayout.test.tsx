import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Navigate, Outlet, Route, Routes, useLocation } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as apiClient from '@/api/client';
import ProjectCatalogLayout from '@/pages/catalog/ProjectCatalogLayout';
import { MovedRedirect } from '@/pages/LegacyRedirects';
import GeneralSettingsPage from '../GeneralSettingsPage';
import MembersSettingsPage from '../MembersSettingsPage';
import NotificationSettingsPage from '../NotificationSettingsPage';
import ProjectSettingsLayout from '../ProjectSettingsLayout';

vi.mock('@/api/client');
vi.mock('@/components/ProjectGeneralSettings', () => ({ default: () => <p>general form</p> }));
vi.mock('@/context/DashboardContext', () => ({
  useDashboard: () => ({
    dashboard: { projects: [{ id: 5, name: 'Space Project' }], user: { id: 1, username: 'alice', is_admin: true } },
    csrfToken: 'csrf',
    refresh: vi.fn(),
  }),
}));

function Where() {
  const { pathname, search } = useLocation();
  return <output data-testid="where">{pathname + search}</output>;
}

/** The project shell provides the outlet context the settings hub expects. */
function Shell() {
  return (
    <>
      <Outlet context={{ projectId: 5, basePath: '/space', globalSearch: '', setGlobalSearch: vi.fn() }} />
      <Where />
    </>
  );
}

function renderAt(path: string) {
  return render(
    <MemoryRouter initialEntries={[path]}>
      <Routes>
        <Route path="/:projectSlug" element={<Shell />}>
          <Route path="settings" element={<ProjectSettingsLayout />}>
            <Route index element={<Navigate to="general" replace />} />
            <Route path="general" element={<GeneralSettingsPage />} />
            <Route path="members" element={<MembersSettingsPage />} />
            <Route path="catalog" element={<ProjectCatalogLayout />}>
              <Route path="categories" element={<p>categories page</p>} />
            </Route>
            <Route path="notifications" element={<NotificationSettingsPage />} />
            <Route path="import" element={<p>import page</p>} />
          </Route>
          <Route path="import" element={<MovedRedirect to="settings/import" />} />
          <Route path="members" element={<MovedRedirect to="settings/members" />} />
          <Route path="catalog/*" element={<MovedRedirect to="settings/catalog" />} />
        </Route>
      </Routes>
    </MemoryRouter>,
  );
}

beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(apiClient.getMyPermissions).mockResolvedValue({
    view_requirements: true,
    edit_requirements: true,
    approve_versions: true,
    is_project_reviewer: true,
    manage_custom_fields: true,
    manage_project_members: true,
    manage_project_configuration: true,
  } as Awaited<ReturnType<typeof apiClient.getMyPermissions>>);
  vi.mocked(apiClient.listProjectMembers).mockResolvedValue([
    { user_id: 2, role: 1, role_label: 'Admin', username: 'dr_smith', name: 'Dr Smith' },
  ]);
  vi.mocked(apiClient.listUsersOptional).mockResolvedValue(null);
  vi.mocked(apiClient.getProjectReviewers).mockResolvedValue({ user_ids: [2] });
  vi.mocked(apiClient.getNotificationPreferences).mockResolvedValue([
    { id: 1, user_id: 1, project_id: 5, notify_in_app: true, notify_email: false },
  ] as Awaited<ReturnType<typeof apiClient.getNotificationPreferences>>);
});

describe('ProjectSettingsLayout', () => {
  it('opens General by default and shows the section tabs', async () => {
    renderAt('/space/settings');
    expect(await screen.findByText('general form')).toBeInTheDocument();
    expect(screen.getByTestId('where')).toHaveTextContent('/space/settings/general');
    const nav = screen.getByRole('navigation', { name: 'Project settings sections' });
    expect(Array.from(nav.querySelectorAll('a')).map((a) => a.getAttribute('href'))).toEqual([
      '/space/settings/general',
      '/space/settings/members',
      '/space/settings/catalog',
      '/space/settings/storage',
      '/space/settings/notifications',
      '/space/settings/import',
    ]);
    expect(screen.getByText('Edit requirements')).toBeInTheDocument();
    // Issue #288: the role capability alone does not grant approvals.
    expect(screen.getByText('Reviewer role capability')).toBeInTheDocument();
    expect(screen.queryByText('Approve versions (role)')).not.toBeInTheDocument();
    expect(screen.getByText('Manage project configuration')).toBeInTheDocument();
    expect(screen.getByText(/needs membership in the\s+project reviewer pool/)).toBeInTheDocument();
  });

  it('loads shared data once while moving between tabs', async () => {
    const user = userEvent.setup();
    renderAt('/space/settings/general');
    await screen.findByText('general form');
    await user.click(screen.getByRole('link', { name: /members & reviewers/i }));
    expect(await screen.findByText('Project reviewers')).toBeInTheDocument();
    expect(screen.getAllByText('Dr Smith (dr_smith)')).toHaveLength(2); // members table + reviewer list
    await user.click(screen.getByRole('link', { name: /notifications/i }));
    await waitFor(() =>
      expect(screen.getByLabelText('In-app notifications for this project')).toBeChecked(),
    );
    expect(apiClient.getMyPermissions).toHaveBeenCalledTimes(1);
    expect(apiClient.listProjectMembers).toHaveBeenCalledTimes(1);
  });

  it('tells non-admin managers who can add people, without the old classic link', async () => {
    renderAt('/space/settings/members');
    expect(await screen.findByText(/visible to instance administrators only/i)).toBeInTheDocument();
    expect(screen.queryByText(/classic/i)).not.toBeInTheDocument();
  });

  it.each([
    ['/space/members', '/space/settings/members'],
    ['/space/import', '/space/settings/import'],
    ['/space/catalog/categories', '/space/settings/catalog/categories'],
  ])('redirects the old %s URL to %s', async (from, to) => {
    renderAt(from);
    await waitFor(() => expect(screen.getByTestId('where')).toHaveTextContent(to));
  });
});
