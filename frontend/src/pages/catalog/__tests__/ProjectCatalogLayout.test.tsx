import { render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter, Route, Routes, useOutletContext } from 'react-router-dom';
import { describe, expect, it, vi } from 'vitest';
import { ThemeProvider } from '@/context/ThemeContext';
import ProjectCatalogLayout from '../ProjectCatalogLayout';
import type { ProjectOutletContext } from '@/types/projectOutlet';

vi.mock('@/context/DashboardContext', () => ({
  useDashboard: () => ({
    dashboard: {
      projects: [{ id: 5, name: 'Space Project' }],
    },
    csrfToken: 'csrf',
  }),
}));

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual<typeof import('react-router-dom')>('react-router-dom');
  return {
    ...actual,
    useOutletContext: vi.fn(),
  };
});

describe('ProjectCatalogLayout', () => {
  it('renders catalog tabs and project name', () => {
    vi.mocked(useOutletContext).mockReturnValue({
      projectId: 5,
      basePath: '/alice/space',
      globalSearch: '',
      setGlobalSearch: vi.fn(),
    } satisfies ProjectOutletContext);

    render(
      <ThemeProvider>
        <MemoryRouter initialEntries={['/alice/space/catalog/categories']}>
          <Routes>
            <Route path="/:ns/:slug/catalog" element={<ProjectCatalogLayout />}>
              <Route path="categories" element={<div>Categories outlet</div>} />
            </Route>
          </Routes>
        </MemoryRouter>
      </ThemeProvider>,
    );

    expect(screen.getByRole('heading', { name: /project catalog/i })).toBeInTheDocument();
    expect(screen.getByText('Space Project')).toBeInTheDocument();
    expect(screen.getByRole('link', { name: /categories/i })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: /custom fields/i })).toBeInTheDocument();
    expect(screen.getByText('Categories outlet')).toBeInTheDocument();
  });
});
