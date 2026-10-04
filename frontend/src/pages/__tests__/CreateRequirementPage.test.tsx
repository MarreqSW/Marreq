import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes, useOutletContext } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as apiClient from '@/api/client';
import type { ProjectOutletContext } from '@/types/projectOutlet';
import { readDraft, requirementDraftKey, writeDraft } from '@/utils/requirementDraft';
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
    localStorage.clear();
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

  it('explains why creating is blocked when the project has no reviewers', async () => {
    vi.mocked(apiClient.getProjectReviewers).mockResolvedValue({ user_ids: [] });
    render(
      <MemoryRouter initialEntries={['/space-project/requirements/new']}>
        <Routes>
          <Route path="/:projectSlug/requirements/new" element={<CreateRequirementPage />} />
        </Routes>
      </MemoryRouter>,
    );

    expect(
      await screen.findByText(/no reviewers, so requirements cannot be created/i),
    ).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /create requirement/i })).toBeDisabled();
    expect(
      screen.getAllByRole('link', { name: /project settings/i })[0],
    ).toHaveAttribute('href', '/space-project/settings/members');
  });

  it('prefills from ?template= the same way as ?from=', async () => {
    render(
      <MemoryRouter initialEntries={['/space-project/requirements/new?template=1']}>
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
    expect(screen.getByText('REQ-SYS-001')).toBeInTheDocument();
  });

  it('prefills a parent link from ?parent= and includes it on create', async () => {
    const user = userEvent.setup();
    render(
      <MemoryRouter initialEntries={['/space-project/requirements/new?parent=2']}>
        <Routes>
          <Route
            path="/:projectSlug/requirements/new"
            element={<CreateRequirementPage />}
          />
        </Routes>
      </MemoryRouter>,
    );

    expect(await screen.findByText('REQ-SYS-001')).toBeInTheDocument();
    expect(screen.queryByRole('status')).not.toBeInTheDocument();

    await user.type(screen.getByPlaceholderText('REQ-0001'), 'REQ-CHILD-001');
    await user.type(screen.getByPlaceholderText('Short title'), 'Child of system');
    await user.type(screen.getByPlaceholderText('Requirement statement…'), 'Shall derive.');
    await user.click(screen.getByRole('button', { name: /create requirement/i }));

    await waitFor(() =>
      expect(apiClient.createRequirementByProject).toHaveBeenCalledWith(
        5,
        expect.objectContaining({
          title: 'Child of system',
          reference_code: 'REQ-CHILD-001',
          parent_links: [
            {
              target_version_id: 22,
              link_type: 'derives-from',
              rationale: null,
            },
          ],
        }),
        'csrf-test',
      ),
    );
  });

  it('warns and skips parent prefill when ?parent= is not in the project', async () => {
    render(
      <MemoryRouter initialEntries={['/space-project/requirements/new?parent=99']}>
        <Routes>
          <Route
            path="/:projectSlug/requirements/new"
            element={<CreateRequirementPage />}
          />
        </Routes>
      </MemoryRouter>,
    );

    expect(await screen.findByRole('status')).toHaveTextContent(
      'Parent requirement 99 is not in this project',
    );
    expect(screen.getByText('No parent requirements selected.')).toBeInTheDocument();
  });

  describe('local draft (issue #255)', () => {
    const KEY = requirementDraftKey(7, 5, 'new');

    function renderNew(path = '/space-project/requirements/new') {
      return render(
        <MemoryRouter initialEntries={[path]}>
          <Routes>
            <Route path="/:projectSlug/requirements/new" element={<CreateRequirementPage />} />
            <Route path="/:projectSlug/requirements/:id/edit" element={<p>edit page</p>} />
          </Routes>
        </MemoryRouter>,
      );
    }

    function storeDraft() {
      writeDraft(KEY, {
        savedAt: new Date().toISOString(),
        baseVersionId: null,
        values: {
          title: 'Battery autonomy',
          description: 'The battery shall last 90 minutes in eclipse.',
          referenceCode: 'REQ-PWR-010',
          justification: 'Longest eclipse',
          categoryId: 11,
          applicabilityId: 12,
          reviewerId: 9,
          methodIds: [14, 999],
          customFieldValues: { 21: 'High', 404: 'gone' },
          parentLinks: [
            { target_version_id: 22, link_type: 'derives-from', rationale: null },
            { target_version_id: 777, link_type: 'derives-from', rationale: null },
          ],
        },
      });
    }

    it('offers a stored draft without applying it, then restores it and creates from it', async () => {
      const user = userEvent.setup();
      storeDraft();
      renderNew();
      expect(await screen.findByText(/unsaved new requirement from .*Battery autonomy/)).toBeInTheDocument();
      expect(screen.getByPlaceholderText('Short title')).toHaveValue('');

      await user.click(screen.getByRole('button', { name: 'Restore draft' }));
      expect(screen.getByPlaceholderText('Short title')).toHaveValue('Battery autonomy');
      expect(screen.getByPlaceholderText('REQ-0001')).toHaveValue('REQ-PWR-010');
      expect(screen.getByLabelText('Rationale (optional)')).toHaveValue('Longest eclipse');

      await user.click(screen.getByRole('button', { name: /create requirement/i }));
      await waitFor(() =>
        expect(apiClient.createRequirementByProject).toHaveBeenCalledWith(
          5,
          expect.objectContaining({
            title: 'Battery autonomy',
            description: 'The battery shall last 90 minutes in eclipse.',
            verification_method_ids: [14],
            custom_fields: [{ field_id: 21, value: 'High' }],
            parent_links: [{ target_version_id: 22, link_type: 'derives-from', rationale: null }],
          }),
          'csrf-test',
        ),
      );
      expect(await screen.findByText('edit page')).toBeInTheDocument();
      expect(readDraft(KEY)).toBeNull();
    });

    it('discards a stored draft', async () => {
      const user = userEvent.setup();
      storeDraft();
      renderNew();
      await user.click(await screen.findByRole('button', { name: 'Discard draft' }));
      expect(screen.queryByRole('region', { name: 'Unsaved draft' })).not.toBeInTheDocument();
      expect(readDraft(KEY)).toBeNull();
    });

    it('keeps what is typed, and offers nothing when there is no draft', async () => {
      const user = userEvent.setup();
      renderNew();
      await user.type(await screen.findByPlaceholderText('Short title'), 'Thermal margin');
      expect(screen.queryByRole('region', { name: 'Unsaved draft' })).not.toBeInTheDocument();
      await waitFor(() =>
        expect(readDraft<{ title: string }>(KEY)?.values.title).toBe('Thermal margin'),
      );
      expect(screen.getByTestId('draft-status')).toHaveTextContent('Draft kept on this device');
    });

    it('does not take an untouched copy for a draft', async () => {
      renderNew('/space-project/requirements/new?from=1');
      expect(await screen.findByDisplayValue('Power mode (Copy)')).toBeInTheDocument();
      expect(screen.getByTestId('draft-status')).toHaveTextContent('');
      expect(readDraft(KEY)).toBeNull();
    });
  });
});
