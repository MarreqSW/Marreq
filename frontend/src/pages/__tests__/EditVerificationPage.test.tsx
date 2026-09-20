import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes, useOutletContext } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as apiClient from '@/api/client';
import type { ProjectOutletContext } from '@/types/projectOutlet';
import EditVerificationPage from '../EditVerificationPage';

vi.mock('@/api/client');

vi.mock('@/context/DashboardContext', () => ({
  useDashboard: () => ({
    csrfToken: 'csrf-test',
    dashboard: {
      user: { id: 7, username: 'author' },
      projects: [{ id: 5, name: 'Space Project' }],
    },
    refresh: vi.fn().mockResolvedValue(undefined),
  }),
}));

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual<typeof import('react-router-dom')>('react-router-dom');
  return {
    ...actual,
    useOutletContext: vi.fn(),
  };
});

describe('EditVerificationPage method', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(useOutletContext).mockReturnValue({
      projectId: 5,
      basePath: '/space-project',
      globalSearch: '',
      setGlobalSearch: vi.fn(),
    } satisfies ProjectOutletContext);
    vi.mocked(apiClient.getMyPermissions).mockResolvedValue({
      view_requirements: true,
      edit_requirements: true,
      approve_versions: true,
      is_project_reviewer: true,
      manage_custom_fields: true,
      manage_project_members: true,
    });
    vi.mocked(apiClient.getVerification).mockResolvedValue({
      id: 42,
      name: 'Power test',
      reference_code: 'VER-010',
      description: 'Measure 500W',
      source: 'manual',
      status_id: 13,
      parent_id: null,
      project_id: 5,
      verification_method_id: 14,
      author_id: 7,
      reviewer_id: 9,
    });
    vi.mocked(apiClient.listVerificationStatuses).mockResolvedValue([
      {
        id: 13,
        title: 'Not run',
        description: '',
        tag: 'NR',
        project_id: 5,
        is_system: true,
        tag_color: null,
      },
    ]);
    vi.mocked(apiClient.listVerificationMethodsByProject).mockResolvedValue([
      { id: 14, title: 'Analysis', description: '', tag: 'A', project_id: 5 },
      { id: 15, title: 'Test', description: '', tag: 'T', project_id: 5 },
    ]);
    vi.mocked(apiClient.listVerifications).mockResolvedValue([]);
    vi.mocked(apiClient.listRequirements).mockResolvedValue([]);
    vi.mocked(apiClient.getVerificationMatrix).mockResolvedValue({
      verification_id: 42,
      requirement_ids: [11],
    });
    vi.mocked(apiClient.listProjectMembers).mockResolvedValue([
      { user_id: 7, role: 3, role_label: 'Author', username: 'author', name: 'Author' },
      { user_id: 9, role: 2, role_label: 'Reviewer', username: 'reviewer', name: 'Reviewer' },
    ]);
    vi.mocked(apiClient.getProjectReviewers).mockResolvedValue({ user_ids: [9] });
    vi.mocked(apiClient.listUsersOptional).mockResolvedValue(null);
    vi.mocked(apiClient.listVerificationSnapshotsByProject).mockResolvedValue([]);
    vi.mocked(apiClient.updateVerificationField).mockResolvedValue(undefined);
    vi.mocked(apiClient.putVerificationMatrix).mockResolvedValue({
      status: 'ok',
      verification_id: 42,
      requirement_ids: [11],
    });
  });

  it('saves a catalog method without rewriting matrix links', async () => {
    const user = userEvent.setup();
    render(
      <MemoryRouter initialEntries={['/space-project/verifications/42/edit']}>
        <Routes>
          <Route path="/space-project/verifications/:verificationId/edit" element={<EditVerificationPage />} />
        </Routes>
      </MemoryRouter>,
    );

    expect(await screen.findByDisplayValue('VER-010')).toBeInTheDocument();
    const methodSelect = screen.getByLabelText(/verification method/i);
    expect(methodSelect).toHaveValue('14');

    await user.selectOptions(methodSelect, '15');
    await user.click(screen.getByRole('button', { name: /save changes/i }));

    await waitFor(() =>
      expect(apiClient.updateVerificationField).toHaveBeenCalledWith(
        5,
        42,
        'verification_method_id',
        '15',
        'csrf-test',
      ),
    );
    expect(apiClient.putVerificationMatrix).not.toHaveBeenCalled();
  });
});
