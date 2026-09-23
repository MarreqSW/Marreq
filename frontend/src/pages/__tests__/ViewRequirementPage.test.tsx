import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes, useOutletContext } from 'react-router-dom';
import { beforeEach, afterEach, describe, expect, it, vi } from 'vitest';
import * as apiClient from '@/api/client';
import type { ProjectOutletContext } from '@/types/projectOutlet';
import type { RequirementDetailPayload, RequirementVersion } from '@/api/types';
import ViewRequirementPage from '../ViewRequirementPage';

vi.mock('@/api/client');

vi.mock('@/context/DashboardContext', () => ({
  useDashboard: () => ({
    csrfToken: 'csrf-test',
    dashboard: {
      user: { id: 9, username: 'reviewer' },
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

const current: RequirementDetailPayload = {
  id: 42,
  current_version_id: 102,
  title: 'Current title',
  description: 'Current statement',
  status_id: 13,
  author_id: 7,
  reviewer_id: 9,
  reference_code: 'REQ-042',
  category_id: 11,
  parent_id: null,
  creation_date: '2026-01-01T00:00:00Z',
  update_date: '2026-03-01T00:00:00Z',
  deadline_date: null,
  applicability_id: 12,
  justification: 'Current rationale',
  project_id: 5,
  approval_state: 'draft',
  approved_by: null,
  approved_at: null,
  custom_fields: [{ field_id: 21, label: 'Priority', value: 'Medium' }],
  verification_method_ids: [14],
  trace_summary: { parent_links: [], child_ids: [99], linked_test_ids: [] },
};

const v1: RequirementVersion = {
  id: 101,
  requirement_id: 42,
  title: 'Approved title',
  description: 'Approved statement',
  status_id: 13,
  author_id: 7,
  reviewer_id: 9,
  category_id: 11,
  applicability_id: 12,
  justification: 'Approved rationale',
  deadline_date: null,
  created_at: '2026-02-01T00:00:00Z',
  approval_state: 'approved',
  approved_by: 9,
  approved_at: '2026-02-02T00:00:00Z',
  custom_fields: [{ field_id: 21, label: 'Priority', value: 'High' }],
  verification_method_ids: [14],
};

const v2: RequirementVersion = {
  ...v1,
  id: 102,
  title: 'Current title',
  description: 'Current statement',
  justification: 'Current rationale',
  created_at: '2026-03-01T00:00:00Z',
  approval_state: 'draft',
  approved_by: null,
  approved_at: null,
  custom_fields: [{ field_id: 21, label: 'Priority', value: 'Medium' }],
};

function renderView(path: string) {
  return render(
    <MemoryRouter initialEntries={[path]}>
      <Routes>
        <Route path="/:projectSlug/requirements/:requirementId/versions/:versionId" element={<ViewRequirementPage />} />
        <Route path="/:projectSlug/requirements/:requirementId" element={<ViewRequirementPage />} />
      </Routes>
    </MemoryRouter>,
  );
}

describe('ViewRequirementPage snapshot', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(useOutletContext).mockReturnValue({
      projectId: 5,
      basePath: '/space-project',
      globalSearch: '',
      setGlobalSearch: vi.fn(),
    } satisfies ProjectOutletContext);
    vi.mocked(apiClient.getRequirementByProject).mockResolvedValue(current);
    vi.mocked(apiClient.listRequirementVersionsByProject).mockResolvedValue([v2, v1]);
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
    vi.mocked(apiClient.listRequirements).mockResolvedValue([current]);
    vi.mocked(apiClient.listVerifications).mockResolvedValue([]);
    vi.mocked(apiClient.listUsersOptional).mockResolvedValue(null);
    vi.mocked(apiClient.listProjectMembers).mockResolvedValue([]);
    vi.mocked(apiClient.listRequirementComments).mockResolvedValue([]);
    vi.mocked(apiClient.getMyPermissions).mockResolvedValue({
      view_requirements: true,
      edit_requirements: true,
      approve_versions: true,
      is_project_reviewer: true,
      manage_custom_fields: true,
      manage_project_members: true,
    });
    vi.mocked(apiClient.listRequirementActivityByProject).mockResolvedValue([]);
    vi.mocked(apiClient.listVerificationMethodsByProject).mockResolvedValue([
      { id: 14, title: 'Test', description: '', tag: 'TEST', project_id: 5 },
    ]);
    vi.mocked(apiClient.getRequirementVersionByProject).mockResolvedValue(v1);
    vi.mocked(apiClient.listRequirementVersionLinks).mockResolvedValue([]);
    vi.mocked(apiClient.createRequirementComment).mockResolvedValue({
      id: 77,
      requirement_id: 42,
      requirement_version_id: 102,
      author_id: 9,
      author_name: 'Reviewer',
      body: 'Need a clarification on power.',
      created_at: '2026-03-02T00:00:00Z',
    });
    vi.mocked(apiClient.setRequirementVersionApproval).mockResolvedValue(v2);
    vi.spyOn(window, 'confirm').mockReturnValue(true);
  });

  afterEach(() => {
    vi.mocked(window.confirm).mockRestore();
  });

  it('links changelog rows to version snapshots on the current view', async () => {
    renderView('/space-project/requirements/42');

    await waitFor(() => expect(screen.getByRole('heading', { name: 'Current title' })).toBeInTheDocument());
    expect(screen.getByRole('link', { name: 'edit Edit' })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'v1' })).toHaveAttribute(
      'href',
      '/space-project/requirements/42/versions/101',
    );
    expect(screen.getByRole('link', { name: 'v2' })).toHaveAttribute(
      'href',
      '/space-project/requirements/42/versions/102',
    );
    expect(screen.getByRole('heading', { name: 'Rationale' })).toBeInTheDocument();
    expect(screen.getByText('Current rationale')).toBeInTheDocument();
  });

  it('shows a read-only historical snapshot without edit actions', async () => {
    renderView('/space-project/requirements/42/versions/101');

    await waitFor(() => expect(screen.getByRole('heading', { name: 'Approved title' })).toBeInTheDocument());
    expect(screen.getByRole('status')).toHaveTextContent(/historical snapshot/i);
    expect(screen.getByRole('link', { name: /view current/i })).toHaveAttribute(
      'href',
      '/space-project/requirements/42',
    );
    expect(screen.queryByRole('link', { name: 'edit Edit' })).not.toBeInTheDocument();
    expect(screen.getByText('Approved statement')).toBeInTheDocument();
    expect(screen.getByText('Approved rationale')).toBeInTheDocument();
    expect(screen.getAllByText('High').length).toBeGreaterThan(0);
    expect(screen.queryByText('Child requirements')).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /add comment/i })).not.toBeInTheDocument();
    expect(screen.queryByLabelText('Add a comment')).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /mark as reviewed/i })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /approve requirement/i })).not.toBeInTheDocument();
  });

  it('lets a reviewer add a comment without edit permission', async () => {
    vi.mocked(apiClient.getMyPermissions).mockResolvedValue({
      view_requirements: true,
      edit_requirements: false,
      approve_versions: false,
      is_project_reviewer: true,
      manage_custom_fields: false,
      manage_project_members: false,
    });
    const user = userEvent.setup();
    renderView('/space-project/requirements/42');

    const box = await screen.findByLabelText('Add a comment');
    await user.type(box, 'Need a clarification on power.');
    await user.click(screen.getByRole('button', { name: /add comment/i }));

    await waitFor(() =>
      expect(apiClient.createRequirementComment).toHaveBeenCalledWith(
        42,
        { body: 'Need a clarification on power.', requirement_version_id: 102 },
        'csrf-test',
      ),
    );
    expect(await screen.findByText('Need a clarification on power.')).toBeInTheDocument();
    expect(screen.queryByRole('link', { name: 'edit Edit' })).not.toBeInTheDocument();
  });

  it('hides the composer when the current version is approved', async () => {
    vi.mocked(apiClient.getRequirementByProject).mockResolvedValue({
      ...current,
      approval_state: 'approved',
      approved_by: 9,
      approved_at: '2026-03-02T00:00:00Z',
    });
    vi.mocked(apiClient.listRequirementVersionsByProject).mockResolvedValue([
      { ...v2, approval_state: 'approved', approved_by: 9, approved_at: '2026-03-02T00:00:00Z' },
      v1,
    ]);
    renderView('/space-project/requirements/42');

    await waitFor(() => expect(screen.getByRole('heading', { name: 'Current title' })).toBeInTheDocument());
    expect(screen.getByText('Comments are locked on this approved version.')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /add comment/i })).not.toBeInTheDocument();
    expect(screen.queryByText('Add a comment in the editor')).not.toBeInTheDocument();
  });

  it('lets a project reviewer mark a draft as reviewed', async () => {
    const user = userEvent.setup();
    renderView('/space-project/requirements/42');

    await user.click(await screen.findByRole('button', { name: /mark as reviewed/i }));

    await waitFor(() =>
      expect(apiClient.setRequirementVersionApproval).toHaveBeenCalledWith(
        5,
        42,
        102,
        'reviewed',
        'csrf-test',
      ),
    );
  });

  it('lets a project reviewer approve a reviewed requirement', async () => {
    vi.mocked(apiClient.getRequirementByProject).mockResolvedValue({
      ...current,
      approval_state: 'reviewed',
    });
    vi.mocked(apiClient.listRequirementVersionsByProject).mockResolvedValue([
      { ...v2, approval_state: 'reviewed' },
      v1,
    ]);
    const user = userEvent.setup();
    renderView('/space-project/requirements/42');

    await user.click(await screen.findByRole('button', { name: /approve requirement/i }));

    await waitFor(() =>
      expect(apiClient.setRequirementVersionApproval).toHaveBeenCalledWith(
        5,
        42,
        102,
        'approved',
        'csrf-test',
      ),
    );
  });

  it('hides approval actions when the user is not a project reviewer', async () => {
    vi.mocked(apiClient.getMyPermissions).mockResolvedValue({
      view_requirements: true,
      edit_requirements: true,
      approve_versions: false,
      is_project_reviewer: false,
      manage_custom_fields: false,
      manage_project_members: false,
    });
    renderView('/space-project/requirements/42');

    await waitFor(() => expect(screen.getByRole('heading', { name: 'Current title' })).toBeInTheDocument());
    expect(screen.queryByRole('button', { name: /mark as reviewed/i })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /approve requirement/i })).not.toBeInTheDocument();
  });
});
