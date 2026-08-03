import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, useOutletContext } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ThemeProvider } from '@/context/ThemeContext';
import * as apiClient from '@/api/client';
import CatalogRequirementStatusesPage from '../CatalogRequirementStatusesPage';
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
        <CatalogRequirementStatusesPage />
      </MemoryRouter>
    </ThemeProvider>,
  );
}

describe('CatalogRequirementStatusesPage', () => {
  beforeEach(() => {
    vi.resetAllMocks();
  });

  it('shows system vs editable rows and creates a status', async () => {
    vi.mocked(apiClient.listRequirementStatuses)
      .mockResolvedValueOnce([
        {
          id: 1,
          title: 'Draft',
          description: '',
          tag: 'draft',
          project_id: 5,
          is_system: true,
          tag_color: null,
        },
        {
          id: 2,
          title: 'Custom',
          description: '',
          tag: 'custom',
          project_id: 5,
          is_system: false,
          tag_color: '#112233',
        },
      ])
      .mockResolvedValue([
        {
          id: 1,
          title: 'Draft',
          description: '',
          tag: 'draft',
          project_id: 5,
          is_system: true,
          tag_color: null,
        },
        {
          id: 2,
          title: 'Custom',
          description: '',
          tag: 'custom',
          project_id: 5,
          is_system: false,
          tag_color: '#112233',
        },
        {
          id: 3,
          title: 'Review',
          description: '',
          tag: 'rev',
          project_id: 5,
          is_system: false,
          tag_color: null,
        },
      ]);
    vi.mocked(apiClient.getMyPermissions).mockResolvedValue(perms);
    vi.mocked(apiClient.createRequirementStatus).mockResolvedValue({ id: 3 });

    renderPage();
    expect(await screen.findByDisplayValue('Draft')).toBeDisabled();
    expect(screen.getByText('System')).toBeInTheDocument();
    expect(screen.getByDisplayValue('Custom')).not.toBeDisabled();

    await userEvent.type(screen.getByPlaceholderText('Title'), 'Review');
    await userEvent.type(screen.getByPlaceholderText('Tag'), 'rev');
    await userEvent.click(screen.getByRole('button', { name: /add status/i }));

    await waitFor(() =>
      expect(apiClient.createRequirementStatus).toHaveBeenCalledWith(
        expect.objectContaining({ title: 'Review', tag: 'rev', project_id: 5 }),
        'csrf-token',
      ),
    );
  });
});
