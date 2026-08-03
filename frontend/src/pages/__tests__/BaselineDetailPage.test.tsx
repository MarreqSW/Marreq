import { render, screen } from '@testing-library/react';
import { MemoryRouter, Route, Routes, useOutletContext } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ThemeProvider } from '@/context/ThemeContext';
import * as apiClient from '@/api/client';
import BaselineDetailPage from '../BaselineDetailPage';
import type { ProjectOutletContext } from '@/types/projectOutlet';

vi.mock('@/api/client');

vi.mock('@/context/DashboardContext', () => ({
  useDashboard: () => ({
    dashboard: { projects: [{ id: 5, name: 'Space' }] },
  }),
}));

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual<typeof import('react-router-dom')>('react-router-dom');
  return { ...actual, useOutletContext: vi.fn() };
});

describe('BaselineDetailPage', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(useOutletContext).mockReturnValue({
      projectId: 5,
      basePath: '/alice/space',
      globalSearch: '',
      setGlobalSearch: vi.fn(),
    } satisfies ProjectOutletContext);
  });

  it('loads snapshot requirements, verifications, and traceability', async () => {
    vi.mocked(apiClient.getBaseline).mockResolvedValue({
      id: 7,
      project_id: 5,
      name: 'PDR freeze',
      description: 'snap',
      created_at: '2024-01-01T00:00:00',
      created_by: 1,
    });
    vi.mocked(apiClient.getBaselineRequirements).mockResolvedValue([
      {
        id: 1,
        current_version_id: 1,
        title: 'Req A',
        description: '',
        status_id: 1,
        author_id: 1,
        reviewer_id: 1,
        reference_code: 'R-1',
        category_id: 1,
        parent_id: null,
        creation_date: '',
        update_date: '',
        deadline_date: null,
        applicability_id: 1,
        justification: null,
        project_id: 5,
        approval_state: 'not_requested',
        approved_by: null,
        approved_at: null,
      },
    ]);
    vi.mocked(apiClient.getBaselineVerifications).mockResolvedValue([
      {
        baseline_id: 7,
        verification_id: 2,
        name: 'Ver B',
        reference_code: 'V-2',
        description: '',
        source: '',
        status_id: 1,
        parent_id: null,
        project_id: 5,
        verification_method_id: null,
      },
    ]);
    vi.mocked(apiClient.getBaselineTraceability).mockResolvedValue([
      {
        baseline_id: 7,
        requirement_id: 1,
        verification_id: 2,
        suspect: false,
        suspect_at: null,
        suspect_reason: null,
      },
    ]);

    render(
      <ThemeProvider>
        <MemoryRouter initialEntries={['/alice/space/baselines/7']}>
          <Routes>
            <Route path="/:ns/:slug/baselines/:baselineId" element={<BaselineDetailPage />} />
          </Routes>
        </MemoryRouter>
      </ThemeProvider>,
    );

    expect(await screen.findByRole('heading', { name: 'PDR freeze' })).toBeInTheDocument();
    expect(screen.getByText('Requirements in snapshot')).toBeInTheDocument();
    expect(screen.getByText('Verifications in snapshot')).toBeInTheDocument();
    expect(screen.getByText('Traceability rows')).toBeInTheDocument();
  });
});
