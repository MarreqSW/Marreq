import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, useOutletContext } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ThemeProvider } from '@/context/ThemeContext';
import * as apiClient from '@/api/client';
import CreateVerificationPage from '../CreateVerificationPage';
import type { ProjectOutletContext } from '@/types/projectOutlet';

vi.mock('@/api/client');

vi.mock('@/components/RequirementMatrixPicker', () => ({
  RequirementMatrixPicker: () => <div data-testid="matrix-picker" />,
}));

const mockNavigate = vi.fn();
const refreshDashboard = vi.fn().mockResolvedValue(undefined);

vi.mock('@/context/DashboardContext', () => ({
  useDashboard: () => ({
    csrfToken: 'csrf',
    dashboard: {
      user: { id: 1 },
      projects: [{ id: 5, name: 'Space' }],
    },
    refresh: refreshDashboard,
  }),
}));

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual<typeof import('react-router-dom')>('react-router-dom');
  return {
    ...actual,
    useOutletContext: vi.fn(),
    useNavigate: () => mockNavigate,
  };
});

const perms = {
  view_requirements: true,
  edit_requirements: true,
  approve_versions: false,
  is_project_reviewer: true,
  manage_custom_fields: false,
  manage_project_members: false,
};

describe('CreateVerificationPage', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(useOutletContext).mockReturnValue({
      projectId: 5,
      basePath: '/alice/space',
      globalSearch: '',
      setGlobalSearch: vi.fn(),
    } satisfies ProjectOutletContext);

    vi.mocked(apiClient.listVerificationStatuses).mockResolvedValue([
      {
        id: 1,
        title: 'not run',
        description: '',
        tag: 'nr',
        project_id: 5,
        is_system: true,
        tag_color: null,
      },
    ]);
    vi.mocked(apiClient.listVerificationMethodsByProject).mockResolvedValue([]);
    vi.mocked(apiClient.listVerifications).mockResolvedValue([]);
    vi.mocked(apiClient.listRequirements).mockResolvedValue([]);
    vi.mocked(apiClient.listProjectMembers).mockResolvedValue([
      { user_id: 1, username: 'alice', name: 'Alice', role: 1, role_label: 'owner' },
    ]);
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
    vi.mocked(apiClient.getProjectReviewers).mockResolvedValue({ user_ids: [1] });
    vi.mocked(apiClient.getMyPermissions).mockResolvedValue(perms);
    vi.mocked(apiClient.createVerification).mockResolvedValue({ id: 88 });
    vi.mocked(apiClient.putVerificationMatrix).mockResolvedValue({
      status: 'ok',
      verification_id: 88,
      requirement_ids: [],
    } as never);
  });

  it('creates a verification and navigates to the detail page', async () => {
    render(
      <ThemeProvider>
        <MemoryRouter>
          <CreateVerificationPage />
        </MemoryRouter>
      </ThemeProvider>,
    );

    expect(await screen.findByRole('heading', { name: /create verification/i })).toBeInTheDocument();
    expect(screen.getByTestId('matrix-picker')).toBeInTheDocument();

    await userEvent.type(screen.getByPlaceholderText('VER-0001'), 'VER-88');
    await userEvent.type(screen.getByPlaceholderText('Verification title'), 'New check');
    await userEvent.type(screen.getByPlaceholderText(/What is being verified/i), 'Details');
    await userEvent.click(screen.getByRole('button', { name: /^create verification$/i }));

    await waitFor(() =>
      expect(apiClient.createVerification).toHaveBeenCalledWith(
        expect.objectContaining({
          reference_code: 'VER-88',
          name: 'New check',
          description: 'Details',
          project_id: 5,
          author_id: 1,
        }),
        'csrf',
      ),
    );
    expect(mockNavigate).toHaveBeenCalledWith('/alice/space/verifications/88');
  });
});
