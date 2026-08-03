import { render, screen } from '@testing-library/react';
import { MemoryRouter, Route, Routes, useOutletContext } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ThemeProvider } from '@/context/ThemeContext';
import * as apiClient from '@/api/client';
import ViewVerificationPage from '../ViewVerificationPage';
import type { ProjectOutletContext } from '@/types/projectOutlet';

vi.mock('@/api/client');

vi.mock('@/context/DashboardContext', () => ({
  useDashboard: () => ({
    dashboard: { projects: [{ id: 5, name: 'Space' }] },
  }),
}));

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual<typeof import('react-router-dom')>('react-router-dom');
  return { ...actual, useOutletContext: vi.fn() };
});

const perms = {
  view_requirements: true,
  edit_requirements: true,
  approve_versions: false,
  is_project_reviewer: false,
  manage_custom_fields: false,
  manage_project_members: false,
};

function mockLoaded() {
  vi.mocked(apiClient.getVerification).mockResolvedValue({
    id: 9,
    name: 'Verify thrust',
    reference_code: 'VER-9',
    description: 'd',
    source: 'lab',
    status_id: 1,
    parent_id: null,
    project_id: 5,
    verification_method_id: null,
    author_id: 1,
    reviewer_id: 1,
  });
  vi.mocked(apiClient.listVerificationStatuses).mockResolvedValue([
    {
      id: 1,
      title: 'Open',
      description: '',
      tag: 'open',
      project_id: 5,
      is_system: true,
      tag_color: null,
    },
  ]);
  vi.mocked(apiClient.listVerificationMethodsByProject).mockResolvedValue([]);
  vi.mocked(apiClient.listVerifications).mockResolvedValue([]);
  vi.mocked(apiClient.getMyPermissions).mockResolvedValue(perms);
  vi.mocked(apiClient.listRequirements).mockResolvedValue([
    {
      id: 42,
      current_version_id: 1,
      title: 'Linked req',
      description: '',
      status_id: 1,
      author_id: 1,
      reviewer_id: 1,
      reference_code: 'REQ-42',
      category_id: 1,
      parent_id: null,
      creation_date: '',
      update_date: '',
      deadline_date: null,
      applicability_id: 1,
      justification: null,
      project_id: 5,
      approval_state: 'not_requested',
      approved_by: null,
      approved_at: null,
    },
  ]);
  vi.mocked(apiClient.getVerificationMatrix).mockResolvedValue({
    verification_id: 9,
    requirement_ids: [42],
  });
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
  vi.mocked(apiClient.listVerificationActivityByProject).mockResolvedValue([
    {
      log_id: 1,
      user_id: 1,
      username: 'alice',
      action_type: 'updated',
      summary: 'Status changed',
      description: null,
      created_at: '2024-01-01T00:00:00',
      changes: [{ field: 'Status', old_value: '1', new_value: '1' }],
    },
  ]);
}

describe('ViewVerificationPage', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(useOutletContext).mockReturnValue({
      projectId: 5,
      basePath: '/alice/space',
      globalSearch: '',
      setGlobalSearch: vi.fn(),
    } satisfies ProjectOutletContext);
  });

  it('shows verification details, linked req, edit CTA, and activity', async () => {
    mockLoaded();
    render(
      <ThemeProvider>
        <MemoryRouter initialEntries={['/alice/space/verifications/9']}>
          <Routes>
            <Route path="/:ns/:slug/verifications/:verificationId" element={<ViewVerificationPage />} />
          </Routes>
        </MemoryRouter>
      </ThemeProvider>,
    );

    expect(await screen.findByText('Verify thrust')).toBeInTheDocument();
    expect(screen.getAllByText('VER-9').length).toBeGreaterThan(0);
    expect(screen.getByText('Linked req')).toBeInTheDocument();
    expect(
      screen.getByRole('link', { name: /edit$/i }),
    ).toHaveAttribute('href', '/alice/space/verifications/9/edit');
    expect(screen.getByText(/Status changed/i)).toBeInTheDocument();
  });

  it('hides edit CTA without edit permission', async () => {
    mockLoaded();
    vi.mocked(apiClient.getMyPermissions).mockResolvedValue({
      ...perms,
      edit_requirements: false,
    });
    render(
      <ThemeProvider>
        <MemoryRouter initialEntries={['/alice/space/verifications/9']}>
          <Routes>
            <Route path="/:ns/:slug/verifications/:verificationId" element={<ViewVerificationPage />} />
          </Routes>
        </MemoryRouter>
      </ThemeProvider>,
    );
    await screen.findByText('Verify thrust');
    expect(screen.queryByRole('link', { name: /^Edit$/i })).not.toBeInTheDocument();
  });
});
