import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes, useOutletContext } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as apiClient from '@/api/client';
import type { ProjectOutletContext } from '@/types/projectOutlet';
import CreateRequirementPage from '../CreateRequirementPage';

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

describe('CreateRequirementPage duplication', () => {
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
    vi.mocked(apiClient.listCategories).mockResolvedValue([
      { id: 11, title: 'General', description: '', tag: 'GEN', project_id: 5 },
    ]);
    vi.mocked(apiClient.listApplicability).mockResolvedValue([
      { id: 12, title: 'All', description: '', tag: 'ALL', project_id: 5 },
    ]);
    vi.mocked(apiClient.listVerificationMethodsByProject).mockResolvedValue([
      { id: 14, title: 'Test', description: '', tag: 'TEST', project_id: 5 },
    ]);
    vi.mocked(apiClient.listProjectMembers).mockResolvedValue([
      { user_id: 7, role: 3, role_label: 'Author', username: 'author', name: 'Author' },
      { user_id: 9, role: 2, role_label: 'Reviewer', username: 'reviewer', name: 'Reviewer' },
    ]);
    vi.mocked(apiClient.getProjectReviewers).mockResolvedValue({ user_ids: [9] });
    vi.mocked(apiClient.listUsersOptional).mockResolvedValue(null);
    vi.mocked(apiClient.listRequirementVersionLinkTypes).mockResolvedValue(['derives-from']);
    vi.mocked(apiClient.listCustomFieldsByProject).mockResolvedValue([
      {
        id: 21,
        project_id: 5,
        label: 'Priority',
        field_type: 'text',
        enum_values: null,
        sort_order: 0,
        created_at: '2026-01-01T00:00:00Z',
      },
    ]);
    vi.mocked(apiClient.listRequirements).mockResolvedValue([
      {
        id: 1,
        current_version_id: 31,
        title: 'Power mode',
        description: 'Source statement',
        status_id: 13,
        author_id: 7,
        reviewer_id: 9,
        reference_code: 'REQ-PWR-001',
        category_id: 11,
        parent_id: 2,
        creation_date: '2026-01-01T00:00:00Z',
        update_date: '2026-01-01T00:00:00Z',
        deadline_date: null,
        applicability_id: 12,
        justification: 'Needed',
        project_id: 5,
        approval_state: 'approved',
        approved_by: 9,
        approved_at: '2026-01-01T00:00:00Z',
      },
      {
        id: 2,
        current_version_id: 22,
        title: 'System',
        description: '',
        status_id: 13,
        author_id: 7,
        reviewer_id: 9,
        reference_code: 'REQ-SYS-001',
        category_id: 11,
        parent_id: null,
        creation_date: '2026-01-01T00:00:00Z',
        update_date: '2026-01-01T00:00:00Z',
        deadline_date: null,
        applicability_id: 12,
        justification: null,
        project_id: 5,
        approval_state: 'draft',
        approved_by: null,
        approved_at: null,
      },
    ]);
    vi.mocked(apiClient.getRequirementByProject).mockResolvedValue({
      id: 1,
      current_version_id: 31,
      title: 'Power mode',
      description: 'Source statement',
      status_id: 13,
      author_id: 7,
      reviewer_id: 9,
      reference_code: 'REQ-PWR-001',
      category_id: 11,
      parent_id: 2,
      creation_date: '2026-01-01T00:00:00Z',
      update_date: '2026-01-01T00:00:00Z',
      deadline_date: null,
      applicability_id: 12,
      justification: 'Needed',
      project_id: 5,
      approval_state: 'approved',
      approved_by: 9,
      approved_at: '2026-01-01T00:00:00Z',
      verification_method_ids: [14],
      custom_fields: [{ field_id: 21, label: 'Priority', value: 'High' }],
      trace_summary: {
        child_ids: [],
        linked_test_ids: [99],
        parent_links: [
          {
            id: 41,
            source_version_id: 31,
            target_version_id: 22,
            link_type: 'derives-from',
            rationale: 'System parent',
            project_id: 5,
            created_at: '2026-01-01T00:00:00Z',
            metadata: null,
          },
        ],
      },
    });
    vi.mocked(apiClient.createRequirementByProject).mockResolvedValue({ id: 3 });
  });

  it('prefills and creates an independent copy without approval or comments', async () => {
    const user = userEvent.setup();
    render(
      <MemoryRouter initialEntries={['/space-project/requirements/new?from=1']}>
        <Routes>
          <Route
            path="/:projectSlug/requirements/new"
            element={<CreateRequirementPage />}
          />
        </Routes>
      </MemoryRouter>,
    );

    expect(await screen.findByDisplayValue('REQ-PWR-002')).toBeInTheDocument();
    expect(screen.getByDisplayValue('Power mode (Copy)')).toBeInTheDocument();
    expect(screen.getByDisplayValue('High')).toBeInTheDocument();
    expect(screen.getByLabelText('Rationale (optional)')).toHaveValue('Needed');

    await user.click(screen.getByRole('button', { name: /create duplicate/i }));

    await waitFor(() =>
      expect(apiClient.createRequirementByProject).toHaveBeenCalledWith(
        5,
        expect.objectContaining({
          title: 'Power mode (Copy)',
          reference_code: 'REQ-PWR-002',
          justification: 'Needed',
          author_id: 7,
          reviewer_id: 9,
          custom_fields: [{ field_id: 21, value: 'High' }],
          parent_links: [
            {
              target_version_id: 22,
              link_type: 'derives-from',
              rationale: 'System parent',
            },
          ],
        }),
        'csrf-test',
      ),
    );
    const payload = vi.mocked(apiClient.createRequirementByProject).mock.calls[0][1];
    expect(payload).not.toHaveProperty('approval_state');
    expect(payload).not.toHaveProperty('comments');
  });
});
