import { render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter, Route, Routes, useOutletContext } from 'react-router-dom';
import { describe, expect, it, vi, beforeEach } from 'vitest';
import { ThemeProvider } from '@/context/ThemeContext';
import HelpPage from '../HelpPage';
import type { ProjectOutletContext } from '@/types/projectOutlet';
import * as apiClient from '@/api/client';

vi.mock('@/api/client');

vi.mock('@/context/DashboardContext', () => ({
  useDashboard: () => ({
    dashboard: {
      projects: [{ id: 5, name: 'Space Project' }],
    },
  }),
}));

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual<typeof import('react-router-dom')>('react-router-dom');
  return {
    ...actual,
    useOutletContext: vi.fn(),
  };
});

const compatibleBuild = {
  backend_version: '0.1.0',
  backend_git_sha: 'abc',
  deployment_mode: 'server',
  frontend_compatibility: { min_version: '0.1.0', max_version: '0.1.99' },
};

describe('HelpPage', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(apiClient.getBuildInfo).mockResolvedValue(compatibleBuild);
  });

  it('renders help sections and shortcut links', async () => {
    vi.mocked(useOutletContext).mockReturnValue({
      projectId: 5,
      basePath: '/space-project',
      globalSearch: '',
      setGlobalSearch: vi.fn(),
    } satisfies ProjectOutletContext);

    render(
      <ThemeProvider>
        <MemoryRouter initialEntries={['/space-project/help']}>
          <Routes>
            <Route path="/:projectSlug/help" element={<HelpPage />} />
          </Routes>
        </MemoryRouter>
      </ThemeProvider>,
    );

    expect(screen.getByRole('heading', { name: /help & reference/i })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: /^navigation$/i })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: /^traceability$/i })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: /dashboard/i })).toHaveAttribute(
      'href',
      '/space-project/dashboard',
    );
    expect(screen.getByRole('link', { name: /reports/i })).toHaveAttribute(
      'href',
      '/space-project/reports',
    );
    expect(screen.getByRole('link', { name: /settings/i })).toHaveAttribute(
      'href',
      '/space-project/settings',
    );

    await waitFor(() =>
      expect(screen.getByTestId('help-compatible')).toHaveTextContent('yes'),
    );
    expect(screen.getByTestId('help-build-info')).toHaveTextContent(/0\.1\.0/);
  });
});
