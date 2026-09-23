import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes, useOutletContext } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as apiClient from '@/api/client';
import type { ProjectOutletContext } from '@/types/projectOutlet';
import EditRequirementPage from '../EditRequirementPage';

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

describe('EditRequirementPage rationale', () => {
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
    vi.mocked(apiClient.getRequirementByProject).mockResolvedValue({
      id: 4,
      current_version_id: 30,
      title: 'Power mode',
      description: 'The system shall provide 500W.',
      status_id: 13,
      author_id: 7,
      reviewer_id: 9,
      reference_code: 'REQ-PWR-001',
      category_id: 11,
      parent_id: null,
      creation_date: '2026-01-01T00:00:00Z',
      update_date: '2026-01-03T00:00:00Z',
      deadline_date: null,
      applicability_id: 12,
      justification: 'Customer power budget',
      project_id: 5,
      approval_state: 'draft',
      approved_by: null,
      approved_at: null,
      verification_method_ids: [14],
      custom_fields: [],
      trace_summary: {
        child_ids: [],
        linked_test_ids: [],
        parent_links: [],
      },
    });
    vi.mocked(apiClient.listRequirementVersionsByProject).mockResolvedValue([]);
    vi.mocked(apiClient.listRequirementStatuses).mockResolvedValue([
      {
        id: 13,
        title: 'Draft',
        description: '',
        tag: 'DRAFT',
        project_id: 5,
        is_system: true,
        tag_color: null,
      },
    ]);
    vi.mocked(apiClient.listVerificationStatuses).mockResolvedValue([]);
    vi.mocked(apiClient.listCategories).mockResolvedValue([
      { id: 11, title: 'General', description: '', tag: 'GEN', project_id: 5 },
    ]);
    vi.mocked(apiClient.listApplicability).mockResolvedValue([
      { id: 12, title: 'All', description: '', tag: 'ALL', project_id: 5 },
    ]);
    vi.mocked(apiClient.listProjectMembers).mockResolvedValue([
      { user_id: 7, role: 3, role_label: 'Author', username: 'author', name: 'Author' },
      { user_id: 9, role: 2, role_label: 'Reviewer', username: 'reviewer', name: 'Reviewer' },
    ]);
    vi.mocked(apiClient.listRequirements).mockResolvedValue([]);
    vi.mocked(apiClient.listVerifications).mockResolvedValue([]);
    vi.mocked(apiClient.listUsersOptional).mockResolvedValue(null);
    vi.mocked(apiClient.listRequirementComments).mockResolvedValue([]);
    vi.mocked(apiClient.listRequirementVersionLinkTypes).mockResolvedValue(['derives-from']);
    vi.mocked(apiClient.getProjectReviewers).mockResolvedValue({ user_ids: [9] });
    vi.mocked(apiClient.patchRequirementByProject).mockResolvedValue(undefined);
  });

  it('patches justification when rationale is edited', async () => {
    const user = userEvent.setup();
    render(
      <MemoryRouter initialEntries={['/space-project/requirements/4/edit']}>
        <Routes>
          <Route
            path="/:projectSlug/requirements/:requirementId/edit"
            element={<EditRequirementPage />}
          />
        </Routes>
      </MemoryRouter>,
    );

    const rationale = await screen.findByLabelText('Rationale (optional)');
    expect(rationale).toHaveValue('Customer power budget');

    await user.clear(rationale);
    await user.type(rationale, 'From analysis');
    await user.click(screen.getByRole('button', { name: /save requirement/i }));

    await waitFor(() =>
      expect(apiClient.patchRequirementByProject).toHaveBeenCalledWith(
        5,
        4,
        { justification: 'From analysis' },
        'csrf-test',
      ),
    );
  });

  it('hides the comment composer when the latest version is approved', async () => {
    vi.mocked(apiClient.getRequirementByProject).mockResolvedValue({
      id: 4,
      current_version_id: 30,
      title: 'Power mode',
      description: 'The system shall provide 500W.',
      status_id: 13,
      author_id: 7,
      reviewer_id: 9,
      reference_code: 'REQ-PWR-001',
      category_id: 11,
      parent_id: null,
      creation_date: '2026-01-01T00:00:00Z',
      update_date: '2026-01-03T00:00:00Z',
      deadline_date: null,
      applicability_id: 12,
      justification: 'Customer power budget',
      project_id: 5,
      approval_state: 'approved',
      approved_by: 9,
      approved_at: '2026-01-03T00:00:00Z',
      verification_method_ids: [14],
      custom_fields: [],
      trace_summary: {
        child_ids: [],
        linked_test_ids: [],
        parent_links: [],
      },
    });
    vi.mocked(apiClient.listRequirementVersionsByProject).mockResolvedValue([
      {
        id: 30,
        requirement_id: 4,
        title: 'Power mode',
        description: 'The system shall provide 500W.',
        status_id: 13,
        author_id: 7,
        reviewer_id: 9,
        category_id: 11,
        applicability_id: 12,
        justification: 'Customer power budget',
        deadline_date: null,
        created_at: '2026-01-03T00:00:00Z',
        approval_state: 'approved',
        approved_by: 9,
        approved_at: '2026-01-03T00:00:00Z',
        custom_fields: [],
        verification_method_ids: [14],
      },
    ]);

    render(
      <MemoryRouter initialEntries={['/space-project/requirements/4/edit']}>
        <Routes>
          <Route
            path="/:projectSlug/requirements/:requirementId/edit"
            element={<EditRequirementPage />}
          />
        </Routes>
      </MemoryRouter>,
    );

    expect(
      await screen.findByText('Comments are locked on this approved version.'),
    ).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /add comment/i })).not.toBeInTheDocument();
  });
});
