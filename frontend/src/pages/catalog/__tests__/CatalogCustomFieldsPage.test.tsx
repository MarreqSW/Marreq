import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, useOutletContext } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ThemeProvider } from '@/context/ThemeContext';
import * as apiClient from '@/api/client';
import CatalogCustomFieldsPage from '../CatalogCustomFieldsPage';
import type { ProjectOutletContext } from '@/types/projectOutlet';

vi.mock('@/api/client');

vi.mock('@/context/DashboardContext', () => ({
  useDashboard: () => ({ csrfToken: 'csrf-token' }),
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
  manage_custom_fields: true,
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
        <CatalogCustomFieldsPage />
      </MemoryRouter>
    </ThemeProvider>,
  );
}

describe('CatalogCustomFieldsPage', () => {
  beforeEach(() => {
    vi.resetAllMocks();
  });

  it('loads enum fields and creates a new enum field', async () => {
    vi.mocked(apiClient.listCustomFieldsByProject)
      .mockResolvedValueOnce([
        {
          id: 1,
          project_id: 5,
          label: 'Priority',
          field_type: 'enum',
          enum_values: ['Low', 'High'],
          sort_order: 0,
          created_at: '',
        },
      ])
      .mockResolvedValue([
        {
          id: 1,
          project_id: 5,
          label: 'Priority',
          field_type: 'enum',
          enum_values: ['Low', 'High'],
          sort_order: 0,
          created_at: '',
        },
        {
          id: 2,
          project_id: 5,
          label: 'Phase',
          field_type: 'enum',
          enum_values: ['A', 'B'],
          sort_order: 1,
          created_at: '',
        },
      ]);
    vi.mocked(apiClient.getMyPermissions).mockResolvedValue(perms);
    vi.mocked(apiClient.createCustomField).mockResolvedValue({ id: 2 });

    renderPage();
    expect(await screen.findByDisplayValue('Priority')).toBeInTheDocument();
    const enumBox = document.querySelector('textarea');
    expect(enumBox?.value.replace(/\r\n/g, '\n')).toBe('Low\nHigh');

    await userEvent.type(screen.getByPlaceholderText('Label'), 'Phase');
    const typeSelects = screen.getAllByRole('combobox');
    await userEvent.selectOptions(typeSelects[0], 'enum');
    await userEvent.type(screen.getByPlaceholderText(/Enum options/i), 'A{Enter}B');
    await userEvent.click(screen.getByRole('button', { name: /add field/i }));

    await waitFor(() =>
      expect(apiClient.createCustomField).toHaveBeenCalledWith(
        5,
        expect.objectContaining({
          label: 'Phase',
          field_type: 'enum',
          enum_values: ['A', 'B'],
        }),
        'csrf-token',
      ),
    );
  });

  it('deletes a field after confirm', async () => {
    vi.mocked(apiClient.listCustomFieldsByProject).mockResolvedValue([
      {
        id: 1,
        project_id: 5,
        label: 'Notes',
        field_type: 'text',
        enum_values: null,
        sort_order: 0,
        created_at: '',
      },
    ]);
    vi.mocked(apiClient.getMyPermissions).mockResolvedValue(perms);
    vi.mocked(apiClient.deleteCustomField).mockResolvedValue(undefined);
    vi.spyOn(window, 'confirm').mockReturnValue(true);

    renderPage();
    await screen.findByDisplayValue('Notes');
    await userEvent.click(screen.getByRole('button', { name: /^delete$/i }));
    await waitFor(() => expect(apiClient.deleteCustomField).toHaveBeenCalledWith(5, 1, 'csrf-token'));
  });
});
