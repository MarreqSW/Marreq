import { cleanup, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { StrictMode } from 'react';
import { MemoryRouter, Route, Routes, useOutletContext } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import * as apiClient from '@/api/client';
import type { RequirementDetailPayload } from '@/api/types';
import type { ProjectOutletContext } from '@/types/projectOutlet';
import { resetApprovedEditPromptsForTests } from '@/utils/confirmEditApprovedRequirement';
import { readDraft, requirementDraftKey, writeDraft } from '@/utils/requirementDraft';
import EditRequirementPage from '../EditRequirementPage';

// Every API function mocked, but the real ApiError so a 401 can be thrown.
vi.mock('@/api/client', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api/client')>();
  const mocked = Object.fromEntries(
    Object.entries(actual).map(([name, value]) => [name, typeof value === 'function' ? vi.fn() : value]),
  );
  return { ...mocked, ApiError: actual.ApiError };
});

vi.mock('@/context/DashboardContext', () => ({
  useDashboard: () => ({
    csrfToken: 'csrf-test',
    dashboard: { user: { id: 7, username: 'author' }, projects: [{ id: 5, name: 'Space Project' }] },
    refresh: vi.fn().mockResolvedValue(undefined),
  }),
}));

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual<typeof import('react-router-dom')>('react-router-dom');
  return { ...actual, useOutletContext: vi.fn() };
});

const LOADED = { timeout: 5000 };
const KEY = requirementDraftKey(7, 5, 4);

const requirement: RequirementDetailPayload = {
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
  trace_summary: { child_ids: [], linked_test_ids: [], parent_links: [] },
};

const draftValues = {
  title: 'Power mode',
  description: 'The system shall provide 500W in every mode, including safe mode.',
  justification: 'Customer power budget',
  statusId: 13,
  categoryId: 11,
  applicabilityId: 12,
  authorId: 7,
  reviewerId: 9,
};

function storeDraft(baseVersionId: number, values = draftValues) {
  writeDraft(KEY, { savedAt: new Date().toISOString(), baseVersionId, values });
}

function renderPage() {
  return render(
    <MemoryRouter initialEntries={['/space-project/requirements/4/edit']}>
      <Routes>
        <Route path="/space-project/requirements/:requirementId/edit" element={<EditRequirementPage />} />
        <Route path="/space-project/requirements" element={<p>requirement list</p>} />
      </Routes>
    </MemoryRouter>,
  );
}

const statement = () => screen.getByLabelText(/statement/i, { selector: 'textarea' });

describe('EditRequirementPage local draft (issue #255)', { timeout: 15_000 }, () => {
  beforeEach(() => {
    vi.resetAllMocks();
    localStorage.clear();
    vi.mocked(apiClient.listAttachments).mockResolvedValue([]);
    vi.mocked(apiClient.getProjectStorage).mockRejectedValue(new Error('not needed'));
    vi.mocked(useOutletContext).mockReturnValue({
      projectId: 5,
      basePath: '/space-project',
      globalSearch: '',
      setGlobalSearch: vi.fn(),
    } satisfies ProjectOutletContext);
    vi.mocked(apiClient.getMyPermissions).mockResolvedValue({
      view_requirements: true,
      edit_requirements: true,
      approve_versions: false,
      is_project_reviewer: false,
      manage_custom_fields: false,
      manage_project_members: false,
    });
    vi.mocked(apiClient.getRequirementByProject).mockResolvedValue(requirement);
    vi.mocked(apiClient.listRequirementVersionsByProject).mockResolvedValue([]);
    vi.mocked(apiClient.listRequirementStatuses).mockResolvedValue([
      { id: 13, title: 'Draft', description: '', tag: 'DRAFT', project_id: 5, is_system: true, tag_color: null },
      { id: 15, title: 'Accepted', description: '', tag: 'ACC', project_id: 5, is_system: true, tag_color: null },
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
    resetApprovedEditPromptsForTests();
    window.confirm = vi.fn(() => true);
  });

  afterEach(() => {
    cleanup();
    localStorage.clear();
    resetApprovedEditPromptsForTests();
  });

  it('keeps typed text as a local draft and shows it in the footer', async () => {
    const user = userEvent.setup();
    renderPage();
    await screen.findByDisplayValue('Power mode', {}, LOADED);
    await user.type(statement(), ' Always.');
    expect(screen.getByTestId('draft-status')).toHaveTextContent('Unsaved changes');
    await waitFor(() => expect(screen.getByTestId('draft-status')).toHaveTextContent('Draft kept on this device'), LOADED);
    expect(readDraft<typeof draftValues>(KEY)).toMatchObject({
      baseVersionId: 30,
      values: { description: 'The system shall provide 500W. Always.' },
    });
    expect(apiClient.patchRequirementByProject).not.toHaveBeenCalled();
  });

  it('restores a draft of the same version, and Discard reverts it', async () => {
    const user = userEvent.setup();
    storeDraft(30);
    renderPage();
    expect(await screen.findByText(/Recovered unsaved changes from/, {}, LOADED)).toBeInTheDocument();
    expect(statement()).toHaveValue(draftValues.description);
    expect(screen.getByRole('button', { name: 'Save requirement' })).toBeEnabled();

    await user.click(screen.getByRole('button', { name: 'Discard draft' }));
    expect(statement()).toHaveValue(requirement.description);
    expect(screen.queryByRole('region', { name: 'Unsaved draft' })).not.toBeInTheDocument();
    expect(readDraft(KEY)).toBeNull();
  });

  // A slower, superseded load (StrictMode runs the effect twice) must not
  // overwrite the restored draft; found in the browser, not in plain renders.
  it('keeps a restored draft when an older load finishes last', async () => {
    storeDraft(30);
    let first = true;
    vi.mocked(apiClient.getRequirementByProject).mockImplementation(async () => {
      if (first) {
        first = false;
        await new Promise((resolve) => setTimeout(resolve, 300));
      }
      return requirement;
    });
    render(
      <StrictMode>
        <MemoryRouter initialEntries={['/space-project/requirements/4/edit']}>
          <Routes>
            <Route path="/space-project/requirements/:requirementId/edit" element={<EditRequirementPage />} />
          </Routes>
        </MemoryRouter>
      </StrictMode>,
    );
    await screen.findByText(/Recovered unsaved changes from/, {}, LOADED);
    await new Promise((resolve) => setTimeout(resolve, 500));
    expect(statement()).toHaveValue(draftValues.description);
    expect(screen.getByRole('button', { name: 'Save requirement' })).toBeEnabled();
  });

  it('offers, but does not apply, a draft taken from an older version', async () => {
    const user = userEvent.setup();
    storeDraft(29);
    renderPage();
    expect(await screen.findByText(/has been saved again since/, {}, LOADED)).toBeInTheDocument();
    expect(statement()).toHaveValue(requirement.description);
    expect(readDraft(KEY)).not.toBeNull();

    await user.click(screen.getByRole('button', { name: 'Restore draft' }));
    expect(statement()).toHaveValue(draftValues.description);
  });

  it('ignores a drafted status change when the user is not a reviewer', async () => {
    storeDraft(30, { ...draftValues, statusId: 15 });
    renderPage();
    await screen.findByText(/Recovered unsaved changes from/, {}, LOADED);
    expect(statement()).toHaveValue(draftValues.description);
    await clickSave();
    expect(apiClient.patchRequirementByProject).toHaveBeenCalledWith(
      5,
      4,
      { description: draftValues.description },
      'csrf-test',
    );
  });

  it('drops the draft once the requirement is saved', async () => {
    storeDraft(30);
    renderPage();
    await screen.findByText(/Recovered unsaved changes from/, {}, LOADED);
    vi.mocked(apiClient.getRequirementByProject).mockResolvedValue({
      ...requirement,
      current_version_id: 31,
      description: draftValues.description,
    });
    await clickSave();
    await waitFor(() => expect(screen.queryByRole('region', { name: 'Unsaved draft' })).not.toBeInTheDocument());
    await waitFor(() => expect(statement()).toHaveValue(draftValues.description));
    expect(readDraft(KEY)).toBeNull();
  });

  it('keeps the draft and explains an expired session', async () => {
    storeDraft(30);
    vi.mocked(apiClient.patchRequirementByProject).mockRejectedValue(
      new apiClient.ApiError(401, 'authentication required'),
    );
    renderPage();
    await screen.findByText(/Recovered unsaved changes from/, {}, LOADED);
    await clickSave();
    expect(await screen.findByText(/Your session has expired/)).toBeInTheDocument();
    expect(readDraft(KEY)).not.toBeNull();
  });

  it('asks before Cancel throws changes away, and drops the draft when confirmed', async () => {
    const user = userEvent.setup();
    storeDraft(30);
    renderPage();
    await screen.findByText(/Recovered unsaved changes from/, {}, LOADED);

    window.confirm = vi.fn(() => false);
    await user.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(window.confirm).toHaveBeenCalledWith('Discard unsaved changes?');
    expect(screen.queryByText('requirement list')).not.toBeInTheDocument();

    window.confirm = vi.fn(() => true);
    await user.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(await screen.findByText('requirement list')).toBeInTheDocument();
    expect(readDraft(KEY)).toBeNull();
  });
});

async function clickSave() {
  await userEvent.setup().click(screen.getByRole('button', { name: 'Save requirement' }));
}
