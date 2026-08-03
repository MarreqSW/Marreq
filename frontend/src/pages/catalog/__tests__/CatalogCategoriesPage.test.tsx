import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, useOutletContext } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ThemeProvider } from '@/context/ThemeContext';
import * as apiClient from '@/api/client';
import CatalogCategoriesPage from '../CatalogCategoriesPage';
import type { ProjectOutletContext } from '@/types/projectOutlet';

vi.mock('@/api/client');

const dash = { csrfToken: 'csrf-token' as string | null };

vi.mock('@/context/DashboardContext', () => ({
  useDashboard: () => dash,
}));

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual<typeof import('react-router-dom')>('react-router-dom');
  return {
    ...actual,
    useOutletContext: vi.fn(),
  };
});

const perms = {
  view_requirements: true,
  edit_requirements: true,
  approve_versions: false,
  is_project_reviewer: false,
  manage_custom_fields: false,
  manage_project_members: false,
};

function renderPage() {
  vi.mocked(useOutletContext).mockReturnValue({
    projectId: 5,
    basePath: '/alice/space',
    globalSearch: '',
    setGlobalSearch: vi.fn(),
  } satisfies ProjectOutletContext);

  return render(
    <ThemeProvider>
      <MemoryRouter>
        <CatalogCategoriesPage />
      </MemoryRouter>
    </ThemeProvider>,
  );
}

describe('CatalogCategoriesPage', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    dash.csrfToken = 'csrf-token';
  });

  it('lists categories and creates a new one', async () => {
    vi.mocked(apiClient.listCategories)
      .mockResolvedValueOnce([
        { id: 1, title: 'Functional', description: 'd', tag: 'fn', project_id: 5 },
      ])
      .mockResolvedValue([
        { id: 1, title: 'Functional', description: 'd', tag: 'fn', project_id: 5 },
        { id: 2, title: 'Safety', description: '', tag: 'safe', project_id: 5 },
      ]);
    vi.mocked(apiClient.getMyPermissions).mockResolvedValue(perms);
    vi.mocked(apiClient.createCategory).mockResolvedValue({ id: 2 });

    renderPage();
    expect(await screen.findByDisplayValue('Functional')).toBeInTheDocument();

    await userEvent.type(screen.getByPlaceholderText('Title'), 'Safety');
    await userEvent.type(screen.getByPlaceholderText('Tag'), 'safe');
    await userEvent.click(screen.getByRole('button', { name: /add category/i }));

    await waitFor(() =>
      expect(apiClient.createCategory).toHaveBeenCalledWith(
        expect.objectContaining({ title: 'Safety', tag: 'safe', project_id: 5 }),
        'csrf-token',
      ),
    );
    expect(await screen.findByDisplayValue('Safety')).toBeInTheDocument();
  });

  it('saves and deletes a row', async () => {
    vi.mocked(apiClient.listCategories).mockResolvedValue([
      { id: 1, title: 'Functional', description: 'd', tag: 'fn', project_id: 5 },
    ]);
    vi.mocked(apiClient.getMyPermissions).mockResolvedValue(perms);
    vi.mocked(apiClient.updateCategory).mockResolvedValue(undefined);
    vi.mocked(apiClient.deleteCategory).mockResolvedValue(undefined);
    vi.spyOn(window, 'confirm').mockReturnValue(true);

    renderPage();
    const titleInput = await screen.findByDisplayValue('Functional');
    await userEvent.clear(titleInput);
    await userEvent.type(titleInput, 'Updated');
    await userEvent.click(screen.getByRole('button', { name: /^save$/i }));
    await waitFor(() =>
      expect(apiClient.updateCategory).toHaveBeenCalledWith(
        1,
        expect.objectContaining({ title: 'Updated' }),
        'csrf-token',
      ),
    );

    await userEvent.click(screen.getByRole('button', { name: /^delete$/i }));
    await waitFor(() => expect(apiClient.deleteCategory).toHaveBeenCalledWith(1, 'csrf-token'));
  });

  it('shows read-only notice without edit permission', async () => {
    vi.mocked(apiClient.listCategories).mockResolvedValue([]);
    vi.mocked(apiClient.getMyPermissions).mockResolvedValue({
      ...perms,
      edit_requirements: false,
    });
    renderPage();
    expect(await screen.findByText(/edit requirements/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /add category/i })).toBeDisabled();
  });
});
