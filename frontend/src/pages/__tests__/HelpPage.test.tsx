import { render, screen } from '@testing-library/react';
import { MemoryRouter, Route, Routes, useOutletContext } from 'react-router-dom';
import { describe, expect, it, vi } from 'vitest';
import { ThemeProvider } from '@/context/ThemeContext';
import HelpPage from '../HelpPage';
import type { ProjectOutletContext } from '@/types/projectOutlet';

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

describe('HelpPage', () => {
  it('renders help sections and shortcut links', () => {
    vi.mocked(useOutletContext).mockReturnValue({
      projectId: 5,
      basePath: '/alice/space-project',
      globalSearch: '',
      setGlobalSearch: vi.fn(),
    } satisfies ProjectOutletContext);

    render(
      <ThemeProvider>
        <MemoryRouter initialEntries={['/alice/space-project/help']}>
          <Routes>
            <Route path="/:ns/:slug/help" element={<HelpPage />} />
          </Routes>
        </MemoryRouter>
      </ThemeProvider>,
    );

    expect(screen.getByRole('heading', { name: /help & reference/i })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: /^navigation$/i })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: /^traceability$/i })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: /dashboard/i })).toHaveAttribute(
      'href',
      '/alice/space-project/dashboard',
    );
    expect(screen.getByRole('link', { name: /reports/i })).toHaveAttribute(
      'href',
      '/alice/space-project/reports',
    );
    expect(screen.getByRole('link', { name: /settings/i })).toHaveAttribute(
      'href',
      '/alice/space-project/settings',
    );
  });
});
