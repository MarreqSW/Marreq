import { render, screen } from '@testing-library/react';
import { MemoryRouter, useOutletContext } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ThemeProvider } from '@/context/ThemeContext';
import * as apiClient from '@/api/client';
import DashboardPage from '../DashboardPage';
import type { ProjectOutletContext } from '@/types/projectOutlet';

vi.mock('@/api/client');

vi.mock('@/context/DashboardContext', () => ({
  useDashboard: () => ({
    dashboard: { projects: [{ id: 5, name: 'Space Project' }] },
  }),
}));

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual<typeof import('react-router-dom')>('react-router-dom');
  return { ...actual, useOutletContext: vi.fn() };
});

describe('DashboardPage', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(useOutletContext).mockReturnValue({
      projectId: 5,
      basePath: '/alice/space',
      globalSearch: '',
      setGlobalSearch: vi.fn(),
    } satisfies ProjectOutletContext);
  });

  it('renders overview stats from parallel API calls', async () => {
    vi.mocked(apiClient.listRequirements).mockResolvedValue([{ id: 1 }, { id: 2 }] as never);
    vi.mocked(apiClient.listVerifications).mockResolvedValue([
      { id: 1, project_id: 5 },
      { id: 2, project_id: 9 },
    ] as never);
    vi.mocked(apiClient.listMatrix).mockResolvedValue([{ req_id: 1, verification_id: 1 }] as never);
    vi.mocked(apiClient.getCoverageReport).mockResolvedValue({
      requirements_without_tests: [2],
      tests_without_requirements: [],
      suspect_links: [{ req_id: 1, verification_id: 1 }],
    });

    render(
      <ThemeProvider>
        <MemoryRouter>
          <DashboardPage />
        </MemoryRouter>
      </ThemeProvider>,
    );

    expect(await screen.findByRole('heading', { name: /project overview/i })).toBeInTheDocument();
    expect(screen.getByLabelText(/Requirements: 2/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/Verifications: 1/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/Matrix links: 1/i)).toBeInTheDocument();
    expect(screen.getByText(/50%/)).toBeInTheDocument();
    expect(screen.getByRole('link', { name: /Gaps/i })).toHaveAttribute(
      'href',
      '/alice/space/reports#gaps',
    );
  });
});
