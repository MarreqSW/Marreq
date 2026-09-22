import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes, useOutletContext } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as apiClient from '@/api/client';
import type { ProjectOutletContext } from '@/types/projectOutlet';
import CreateVerificationPage from '../CreateVerificationPage';

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

function renderCreate(query: string) {
  return render(
    <MemoryRouter initialEntries={[`/space-project/verifications/new${query}`]}>
      <Routes>
        <Route path="/:projectSlug/verifications/new" element={<CreateVerificationPage />} />
      </Routes>
    </MemoryRouter>,
  );
}

describe('CreateVerificationPage query prefill', () => {
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
      { id: 14, title: 'Test', description: '', tag: 'T', project_id: 5 },
    ]);
    vi.mocked(apiClient.listVerifications).mockResolvedValue([
      {
        id: 10,
        name: 'Parent check',
        reference_code: 'VER-009',
        description: 'Parent',
        source: 'manual',
        status_id: 13,
        parent_id: null,
        project_id: 5,
        verification_method_id: 14,
        author_id: 7,
        reviewer_id: 9,
      },
      {
        id: 11,
        name: 'Power test',
        reference_code: 'VER-010',
        description: 'Measure 500W',
        source: 'lab',
        status_id: 13,
        parent_id: 10,
        project_id: 5,
        verification_method_id: 14,
        author_id: 7,
        reviewer_id: 9,
      },
    ]);
    vi.mocked(apiClient.listRequirements).mockResolvedValue([]);
    vi.mocked(apiClient.listProjectMembers).mockResolvedValue([
      { user_id: 7, role: 3, role_label: 'Author', username: 'author', name: 'Author' },
      { user_id: 9, role: 2, role_label: 'Reviewer', username: 'reviewer', name: 'Reviewer' },
    ]);
    vi.mocked(apiClient.listUsersOptional).mockResolvedValue(null);
    vi.mocked(apiClient.getProjectReviewers).mockResolvedValue({ user_ids: [9] });
    vi.mocked(apiClient.getVerification).mockResolvedValue({
      id: 11,
      name: 'Power test',
      reference_code: 'VER-010',
      description: 'Measure 500W',
      source: 'lab',
      status_id: 13,
      parent_id: 10,
      project_id: 5,
      verification_method_id: 14,
      author_id: 7,
      reviewer_id: 9,
    });
    vi.mocked(apiClient.createVerification).mockResolvedValue({ id: 12 });
  });

  it('copies fields from ?template=', async () => {
    renderCreate('?template=11');

    expect(await screen.findByDisplayValue('VER-011')).toBeInTheDocument();
    expect(screen.getByDisplayValue('Power test (Copy)')).toBeInTheDocument();
    expect(screen.getByDisplayValue('Measure 500W')).toBeInTheDocument();
    expect(screen.getByLabelText('Parent verification (optional)')).toHaveValue('10');
  });

  it('prefills parent from ?parent= and includes it on create', async () => {
    const user = userEvent.setup();
    renderCreate('?parent=10');

    const parentSelect = await screen.findByLabelText('Parent verification (optional)');
    expect(parentSelect).toHaveValue('10');

    await user.type(screen.getByPlaceholderText('VER-0001'), 'VER-020');
    await user.type(screen.getByPlaceholderText('Verification title'), 'Child check');
    await user.type(screen.getByPlaceholderText('What is being verified…'), 'Child statement');
    await user.click(screen.getByRole('button', { name: /create verification/i }));

    await waitFor(() =>
      expect(apiClient.createVerification).toHaveBeenCalledWith(
        expect.objectContaining({
          name: 'Child check',
          reference_code: 'VER-020',
          parent_id: 10,
          project_id: 5,
        }),
        'csrf-test',
      ),
    );
  });

  it('warns when ?parent= is not in the project', async () => {
    renderCreate('?parent=99');

    expect(await screen.findByRole('status')).toHaveTextContent(
      'Parent verification 99 is not in this project',
    );
    expect(screen.getByLabelText('Parent verification (optional)')).toHaveValue('');
  });
});
