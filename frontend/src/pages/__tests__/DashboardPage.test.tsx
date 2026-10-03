import { render, screen } from '@testing-library/react';
import { MemoryRouter, Outlet, Route, Routes } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as apiClient from '@/api/client';
import type { CoverageReport, MatrixLink, Requirement, Verification } from '@/api/types';
import DashboardPage from '../DashboardPage';

vi.mock('@/api/client');
vi.mock('@/context/DashboardContext', () => ({
  useDashboard: () => ({ dashboard: { projects: [{ id: 5, name: 'Satellite Demo' }] } }),
}));

function renderPage() {
  return render(
    <MemoryRouter initialEntries={['/sat/dashboard']}>
      <Routes>
        <Route
          path="/sat"
          element={<Outlet context={{ projectId: 5, basePath: '/sat', globalSearch: '', setGlobalSearch: vi.fn() }} />}
        >
          <Route path="dashboard" element={<DashboardPage />} />
        </Route>
      </Routes>
    </MemoryRouter>,
  );
}

const ids = (n: number) => Array.from({ length: n }, (_, i) => i + 1);

describe('DashboardPage', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(apiClient.listRequirements).mockResolvedValue(ids(4).map((id) => ({ id }) as Requirement));
    vi.mocked(apiClient.listVerifications).mockResolvedValue([
      { id: 1, project_id: 5 } as Verification,
      { id: 2, project_id: 5 } as Verification,
      { id: 3, project_id: 6 } as Verification,
    ]);
    vi.mocked(apiClient.listMatrix).mockResolvedValue([{ req_id: 1 } as MatrixLink]);
    vi.mocked(apiClient.getCoverageReport).mockResolvedValue({
      requirements_without_tests: [2, 3, 4],
      tests_without_requirements: [2],
      suspect_links: [{ req_id: 1, verification_id: 1 }],
    } as CoverageReport);
  });

  it('shows the counts and the traceability health cards', async () => {
    renderPage();
    expect(await screen.findByText('Gaps')).toBeInTheDocument();
    expect(screen.getByText('Req. with tests').parentElement).toHaveTextContent('25%');
    expect(screen.getByRole('link', { name: /Gaps\s*3/ })).toHaveAttribute('href', '/sat/reports#gaps');
    expect(screen.getByRole('link', { name: /Orphans\s*1/ })).toHaveAttribute('href', '/sat/reports#orphans');
    expect(screen.getByRole('link', { name: /Suspect\s*1/ })).toHaveAttribute('href', '/sat/reports#suspect');
  });

  // Issue #360: the status headings were dark-theme shades only and hard to read in light mode.
  it('uses theme-aware shades for the status headings', async () => {
    renderPage();
    const gaps = await screen.findByText('Gaps');
    expect(gaps).toHaveClass('text-amber-800', 'dark:text-amber-200');
    expect(gaps).not.toHaveClass('text-amber-200');
    const suspect = screen.getByText('Suspect');
    expect(suspect).toHaveClass('text-red-700', 'dark:text-red-200');
    expect(suspect).not.toHaveClass('text-red-200');
  });

  it('shows a readable error banner when loading fails', async () => {
    vi.mocked(apiClient.getCoverageReport).mockRejectedValue(new Error('Access denied.'));
    renderPage();
    const banner = await screen.findByText('Access denied.');
    expect(banner).toHaveClass('text-red-800', 'dark:text-red-200');
    expect(banner).not.toHaveClass('text-red-200');
  });
});
