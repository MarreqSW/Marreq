import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { UseCaseDetail, UseCaseDocument } from '@/api/types';
import { ApiError } from '@/api/transport';
import type { ProjectOutletContext } from '@/types/projectOutlet';
import ViewUseCasePage from '../ViewUseCasePage';

const mocks = vi.hoisted(() => ({
  getUseCase: vi.fn(),
  listUseCaseVersions: vi.fn(),
  getMyPermissions: vi.fn(),
  getUseCaseDraft: vi.fn(),
  applyUseCaseOperations: vi.fn(),
  publishUseCaseDraft: vi.fn(),
  discardUseCaseDraft: vi.fn(),
  deleteUseCase: vi.fn(),
  setUseCaseVersionApproval: vi.fn(),
  listUseCaseLinks: vi.fn(),
  createUseCaseLink: vi.fn(),
  deleteUseCaseLink: vi.fn(),
  clearUseCaseLinkSuspect: vi.fn(),
  listRequirements: vi.fn(),
  listVerificationsByProject: vi.fn(),
  getRequirementByProject: vi.fn(),
  listUseCaseComments: vi.fn(),
  createUseCaseComment: vi.fn(),
  listProjectMembers: vi.fn(),
  getUseCaseDraftDiff: vi.fn(),
  getUseCaseVersionDiff: vi.fn(),
}));
vi.mock('@/api/client', async () => ({
  ...mocks,
  ApiError: (await vi.importActual<typeof import('@/api/transport')>('@/api/transport')).ApiError,
}));
vi.mock('@/context/DashboardContext', () => ({
  useDashboard: () => ({ csrfToken: 'csrf', dashboard: { projects: [{ id: 5, name: 'Space Project' }] } }),
}));
vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual<typeof import('react-router-dom')>('react-router-dom');
  return { ...actual, useOutletContext: vi.fn() };
});
import { useOutletContext } from 'react-router-dom';

const doc: UseCaseDocument = {
  schema: 'marreq.use-case.v1',
  title: 'Export requirements',
  primary_actor: 'Engineer',
  secondary_actors: ['Storage service'],
  summary: null,
  preconditions: [],
  main: [
    { id: 's_Open', text: 'User opens the page.' },
    { id: 's_Pick', text: 'User selects requirements.' },
  ],
  branches: [
    {
      id: 'b_None',
      kind: 'alternative',
      title: 'Nothing selected',
      from_step: 's_Pick',
      steps: [{ id: 's_Ask', text: 'System asks again.' }],
      end: { type: 'rejoin', step: 's_Pick' },
    },
  ],
  postconditions: [],
};
const labels = { s_Open: '1', s_Pick: '2', b_None: 'A2', s_Ask: 'A2.1' };

const version = {
  id: 31,
  version_number: 1,
  approval_state: 'draft',
  title: 'Export requirements',
  primary_actor: 'Engineer',
  change_note: null,
  created_by: 1,
  created_by_name: 'Alice Johnson',
  created_at: '2026-10-15T08:00:00',
  reviewed_by: null,
  reviewed_at: null,
  approved_by: null,
  approved_at: null,
  document: doc,
  labels,
};

const detail: UseCaseDetail = {
  id: 7,
  project_id: 5,
  reference_code: 'UC-001',
  title: 'Export requirements',
  created_by: 1,
  created_at: '2026-10-15T08:00:00',
  updated_at: '2026-10-15T08:00:00',
  current_version: version,
  draft: null,
};

function renderPage() {
  vi.mocked(useOutletContext).mockReturnValue({
    projectId: 5,
    basePath: '/space-project',
    globalSearch: '',
    setGlobalSearch: vi.fn(),
  } satisfies ProjectOutletContext);
  return render(
    <MemoryRouter initialEntries={['/space-project/use-cases/7']}>
      <Routes>
        <Route path="/space-project/use-cases/:useCaseId" element={<ViewUseCasePage />} />
      </Routes>
    </MemoryRouter>,
  );
}

function changed(document: UseCaseDocument, revision: number, effects: unknown[] = []) {
  return { use_case_id: 7, revision, base_version_id: 31, updated_by: 1, updated_by_name: 'Alice', updated_at: '2026-10-15T08:00:00', document, labels, effects };
}

describe('ViewUseCasePage', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    mocks.getUseCase.mockResolvedValue(detail);
    mocks.listUseCaseVersions.mockResolvedValue([version]);
    mocks.getMyPermissions.mockResolvedValue({ edit_requirements: true, is_project_reviewer: true });
    mocks.listUseCaseLinks.mockResolvedValue([]);
    mocks.listUseCaseComments.mockResolvedValue([]);
    mocks.listProjectMembers.mockResolvedValue([
      { user_id: 2, role: 2, role_label: 'Author', username: 'dr_smith', name: 'Dr. Smith' },
    ]);
  });

  it('reads the current version with its branches and review action', async () => {
    const user = userEvent.setup();
    renderPage();
    expect(await screen.findByRole('heading', { name: 'Export requirements' })).toBeInTheDocument();
    expect(screen.getByText('User selects requirements.')).toBeInTheDocument();
    expect(screen.getByText('Nothing selected')).toBeInTheDocument();
    expect(screen.getByText('Continues at step 2')).toBeInTheDocument();
    expect(screen.getByText('Storage service')).toBeInTheDocument();

    mocks.setUseCaseVersionApproval.mockResolvedValue({ ...version, approval_state: 'reviewed' });
    await user.click(screen.getByRole('button', { name: 'Mark as reviewed' }));
    expect(mocks.setUseCaseVersionApproval).toHaveBeenCalledWith(5, 7, 31, 'reviewed', 'csrf');
  });

  it('edits from the current version, opening the draft on the first change', async () => {
    const user = userEvent.setup();
    renderPage();
    await user.click(await screen.findByRole('button', { name: /^(edit )?Edit$/ }));
    expect(await screen.findByText(/based on v1/)).toBeInTheDocument();
    expect(mocks.getUseCaseDraft).not.toHaveBeenCalled();

    const edited = { ...doc, main: [doc.main[0]!, { id: 's_Pick', text: 'User picks requirements.' }] };
    mocks.applyUseCaseOperations.mockResolvedValue(changed(edited, 1));
    const step = screen.getByLabelText('Step 2');
    await user.clear(step);
    await user.type(step, 'User picks requirements.');
    await user.tab();
    await waitFor(() =>
      expect(mocks.applyUseCaseOperations).toHaveBeenCalledWith(
        5,
        7,
        null,
        [{ op: 'update_text', id: 's_Pick', text: 'User picks requirements.' }],
        'csrf',
      ),
    );
    expect(await screen.findByText(/all changes saved/)).toBeInTheDocument();

    // Undo sends the inverse against the new revision.
    mocks.applyUseCaseOperations.mockResolvedValue(changed(doc, 2));
    await user.click(screen.getByRole('button', { name: 'Undo' }));
    await waitFor(() =>
      expect(mocks.applyUseCaseOperations).toHaveBeenLastCalledWith(
        5,
        7,
        1,
        [{ op: 'update_text', id: 's_Pick', text: 'User selects requirements.' }],
        'csrf',
      ),
    );
  });

  it('asks what to do with dependent branches before deleting a step', async () => {
    const user = userEvent.setup();
    renderPage();
    await user.click(await screen.findByRole('button', { name: /^(edit )?Edit$/ }));
    mocks.applyUseCaseOperations.mockRejectedValueOnce(
      new ApiError(422, 'has dependents', {
        code: 'element_has_dependents',
        element_id: 's_Pick',
        element_label: '2',
        branches: [{ id: 'b_None', label: 'A2', title: 'Nothing selected' }],
      }),
    );
    const row = screen.getByLabelText('Step 2').closest('[data-element-id]') as HTMLElement;
    await user.click(within(row).getByRole('button', { name: 'More actions' }));
    await user.click(screen.getByRole('menuitem', { name: /Delete step$/ }));
    const dialog = await screen.findByRole('dialog', { name: 'Delete step 2?' });
    expect(within(dialog).getByText(/Nothing selected/)).toBeInTheDocument();

    mocks.applyUseCaseOperations.mockResolvedValue(changed({ ...doc, main: [doc.main[0]!], branches: [] }, 1));
    await user.click(within(dialog).getByRole('radio', { name: /delete it too/i }));
    await user.click(within(dialog).getByRole('button', { name: /delete step/i }));
    await waitFor(() =>
      expect(mocks.applyUseCaseOperations).toHaveBeenLastCalledWith(
        5,
        7,
        null,
        [{ op: 'remove_element', id: 's_Pick', dependents: 'cascade' }],
        'csrf',
      ),
    );
  });

  it('reports a stale draft and publishes the current one', async () => {
    const user = userEvent.setup();
    mocks.getUseCase.mockResolvedValue({
      ...detail,
      draft: { revision: 4, base_version_id: 31, updated_by: 2, updated_by_name: 'Dr. Smith', updated_at: '2026-10-15T08:00:00' },
    });
    mocks.getUseCaseDraft.mockResolvedValue(changed(doc, 4));
    renderPage();
    expect(await screen.findByText(/A draft of the next version is in progress/)).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: /continue editing/i }));
    expect(await screen.findByText(/all changes saved/)).toBeInTheDocument();

    mocks.applyUseCaseOperations.mockRejectedValueOnce(
      new ApiError(409, 'stale', {
        code: 'stale_revision',
        current: { revision: 5, updated_by_name: 'Dr. Smith', updated_at: '2026-10-15T08:00:00' },
      }),
    );
    await user.type(screen.getByLabelText('Step 1'), '!');
    await user.tab();
    expect(await screen.findByText(/Someone else changed this draft \(Dr. Smith/)).toBeInTheDocument();

    mocks.publishUseCaseDraft.mockResolvedValue({ version: { ...version, version_number: 2 } });
    await user.click(screen.getByRole('button', { name: /publish version/i }));
    const dialog = await screen.findByRole('dialog', { name: 'Publish version 2' });
    await user.type(within(dialog).getByRole('textbox'), 'Tightened step 1');
    await user.click(within(dialog).getByRole('button', { name: 'Publish' }));
    await waitFor(() =>
      expect(mocks.publishUseCaseDraft).toHaveBeenCalledWith(5, 7, 4, 'Tightened step 1', 'csrf'),
    );
  });

  it('shows links on their steps, opens a requirement beside the page and links new ones', async () => {
    const user = userEvent.setup();
    const link = {
      id: 3,
      element_id: 's_Pick',
      element_label: '2',
      target_type: 'requirement',
      requirement_id: 11,
      verification_id: null,
      target_reference_code: 'REQ-EXP-004',
      target_title: 'Export a selection',
      rationale: null,
      suspect: true,
      suspect_at: '2026-10-15T08:00:00',
      suspect_reason: 'Requirement updated',
      linked_requirement_version_id: 40,
      current_requirement_version_id: 41,
      created_by: 1,
      created_at: '2026-10-15T08:00:00',
    };
    mocks.listUseCaseLinks.mockResolvedValue([link]);
    mocks.getRequirementByProject.mockResolvedValue({
      id: 11,
      reference_code: 'REQ-EXP-004',
      title: 'Export a selection',
      description: 'The system shall export **selected** requirements.',
      approval_state: 'approved',
    });
    renderPage();

    const panel = await screen.findByText('Related artifacts');
    const aside = panel.closest('div.rounded-lg') as HTMLElement;
    expect(within(aside).getByText('step 2')).toBeInTheDocument();
    expect(within(aside).getByText(/Changed since linked · Requirement updated/)).toBeInTheDocument();

    // The chip on step 2 opens the requirement in a side panel.
    const row = screen.getByText('User selects requirements.').closest('[data-element-id]') as HTMLElement;
    await user.click(within(row).getByRole('button', { name: /REQ-EXP-004/ }));
    const peek = await screen.findByRole('complementary', { name: 'Requirement REQ-EXP-004' });
    expect(await within(peek).findByText('selected')).toBeInTheDocument();
    expect(within(peek).getByRole('link', { name: /open requirement/i })).toHaveAttribute(
      'href',
      '/space-project/requirements/11',
    );
    await user.click(within(peek).getByRole('button', { name: 'Close' }));

    // Clear the suspect flag.
    mocks.clearUseCaseLinkSuspect.mockResolvedValue({ status: 'ok', cleared: true });
    await user.click(within(aside).getByRole('button', { name: 'Clear' }));
    expect(mocks.clearUseCaseLinkSuspect).toHaveBeenCalledWith(5, 7, 3, 'csrf');

    // Link a verification to step 1.
    mocks.listRequirements.mockResolvedValue([]);
    mocks.listVerificationsByProject.mockResolvedValue([
      { id: 21, reference_code: 'TEST-EXP-002', name: 'Export round trip' },
    ]);
    mocks.createUseCaseLink.mockResolvedValue({ ...link, id: 4 });
    await user.click(screen.getByRole('button', { name: 'Link a requirement or verification' }));
    const dialog = await screen.findByRole('dialog', { name: 'Link a requirement or verification' });
    await user.selectOptions(within(dialog).getByRole('combobox'), 's_Open');
    await user.click(within(dialog).getByRole('radio', { name: 'Verification' }));
    await user.click(await within(dialog).findByRole('option', { name: /TEST-EXP-002/ }));
    await user.click(within(dialog).getByRole('button', { name: 'Link' }));
    await waitFor(() =>
      expect(mocks.createUseCaseLink).toHaveBeenCalledWith(
        5,
        7,
        { element_id: 's_Open', verification_id: 21, rationale: null },
        'csrf',
      ),
    );
  });

  it('comments on a step with mentions and reviews the changes between versions', async () => {
    const user = userEvent.setup();
    const v2 = { ...version, id: 32, version_number: 2 };
    mocks.getUseCase.mockResolvedValue({ ...detail, current_version: v2 });
    mocks.listUseCaseVersions.mockResolvedValue([v2, version]);
    mocks.listUseCaseComments.mockResolvedValue([
      {
        id: 1,
        use_case_id: 7,
        use_case_version_id: 31,
        element_id: 's_Pick',
        element_label: '2',
        author_id: 2,
        author_name: 'Dr. Smith',
        body: 'Is @dr_smith right here?',
        created_at: '2026-10-15T08:00:00',
      },
    ]);
    renderPage();

    // The step shows its comment count; clicking it filters the discussion to that step.
    await user.click(await screen.findByRole('button', { name: '1 comment on 2' }));
    const discussion = document.getElementById('discussion') as HTMLElement;
    expect(within(discussion).getByText('On Step 2')).toBeInTheDocument();
    expect(within(discussion).getByTitle('Dr. Smith')).toHaveTextContent('@dr_smith');

    mocks.createUseCaseComment.mockResolvedValue({
      id: 2,
      use_case_id: 7,
      use_case_version_id: 32,
      element_id: 's_Pick',
      element_label: '2',
      author_id: 1,
      author_name: 'Alice',
      body: 'Yes.',
      created_at: '2026-10-15T09:00:00',
    });
    await user.type(within(discussion).getByRole('combobox', { name: 'Add a comment' }), 'Yes.');
    await user.click(within(discussion).getByRole('button', { name: 'Add comment' }));
    expect(mocks.createUseCaseComment).toHaveBeenCalledWith(
      5,
      7,
      { body: 'Yes.', use_case_version_id: 32, element_id: 's_Pick' },
      'csrf',
    );
    expect(await within(discussion).findByText('Yes.')).toBeInTheDocument();

    // Review changes: v1 → v2 with badges and the word diff.
    mocks.getUseCaseVersionDiff.mockResolvedValue({
      from: { version_id: 31, version_number: 1, draft_revision: null },
      to: { version_id: 32, version_number: 2, draft_revision: null },
      summary: { added: 0, removed: 0, edited: 1, moved: 1, rebranched: 0 },
      header: {},
      elements: [
        {
          id: 's_Pick',
          kind: 'step',
          changes: ['edited', 'moved'],
          old: { list: 'main', label: '3', text: 'User picks.' },
          new: { list: 'main', label: '2', text: 'User selects.' },
          text_diff: [
            { op: 'equal', text: 'User ' },
            { op: 'delete', text: 'picks' },
            { op: 'insert', text: 'selects' },
            { op: 'equal', text: '.' },
          ],
        },
      ],
    });
    await user.click(screen.getByRole('tab', { name: /review changes/i }));
    expect(await screen.findByText('1 Edited')).toBeInTheDocument();
    expect(mocks.getUseCaseVersionDiff).toHaveBeenCalledWith(5, 7, 31, 32, false);
    expect(screen.getByText('3 → 2')).toBeInTheDocument();
    expect(screen.getByText('picks').tagName).toBe('DEL');
    expect(screen.getByText('selects').tagName).toBe('INS');
  });
});
