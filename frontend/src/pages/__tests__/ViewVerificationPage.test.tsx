import { render, screen } from '@testing-library/react';
import { MemoryRouter, Route, Routes, useOutletContext } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as apiClient from '@/api/client';
import type { ProjectOutletContext } from '@/types/projectOutlet';
import ViewVerificationPage from '../ViewVerificationPage';

vi.mock('@/api/client');

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

describe('ViewVerificationPage description', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(apiClient.listAttachments).mockResolvedValue([]);
    vi.mocked(apiClient.getProjectStorage).mockRejectedValue(new Error('not needed'));
    vi.mocked(apiClient.getVerificationControl).mockRejectedValue(new Error('not needed'));
    vi.mocked(useOutletContext).mockReturnValue({
      projectId: 5,
      basePath: '/space-project',
      globalSearch: '',
      setGlobalSearch: vi.fn(),
    } satisfies ProjectOutletContext);
    const verification = {
      id: 42,
      name: 'Thermal vacuum',
      reference_code: 'VER-010',
      description: 'Run the **TVAC** cycle:\n1. Pump down\n2. Soak\n\n[Procedure](https://proc.test) [bad](javascript:alert(1))',
      source: 'Lab',
      status_id: 1,
      parent_id: null,
      project_id: 5,
      verification_method_id: null,
      author_id: 7,
      reviewer_id: 7,
    };
    vi.mocked(apiClient.getMyPermissions).mockResolvedValue({
      view_requirements: true,
      edit_requirements: true,
      approve_versions: true,
      is_project_reviewer: true,
      manage_custom_fields: true,
      manage_project_members: true,
    });
    vi.mocked(apiClient.getVerification).mockResolvedValue(verification);
    vi.mocked(apiClient.listVerifications).mockResolvedValue([verification]);
    vi.mocked(apiClient.getVerificationMatrix).mockResolvedValue({ verification_id: 42, requirement_ids: [] });
    vi.mocked(apiClient.listRequirements).mockResolvedValue([]);
    vi.mocked(apiClient.listProjectMembers).mockResolvedValue([]);
    vi.mocked(apiClient.listVerificationActivityByProject).mockResolvedValue([]);
    vi.mocked(apiClient.listVerificationMethodsByProject).mockResolvedValue([]);
    vi.mocked(apiClient.listVerificationSnapshotsByProject).mockResolvedValue([]);
    vi.mocked(apiClient.listVerificationStatuses).mockResolvedValue([]);
    vi.mocked(apiClient.listUsersOptional).mockResolvedValue(null);
  });

  it('renders the formatted description without unsafe links', async () => {
    render(
      <MemoryRouter initialEntries={['/space-project/verifications/42']}>
        <Routes>
          <Route path="/space-project/verifications/:verificationId" element={<ViewVerificationPage />} />
        </Routes>
      </MemoryRouter>,
    );

    const statement = await screen.findByTestId('statement-text');
    expect(statement.querySelector('strong')).toHaveTextContent('TVAC');
    expect(statement.querySelectorAll('ol > li')).toHaveLength(2);
    const links = [...statement.querySelectorAll('a')];
    expect(links.map((a) => a.getAttribute('href'))).toEqual(['https://proc.test']);
    expect(statement).toHaveTextContent('[bad](javascript:alert(1))');
  });
});
