import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, useOutletContext } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ThemeProvider } from '@/context/ThemeContext';
import * as apiClient from '@/api/client';
import BaselinesPage from '../BaselinesPage';
import type { ProjectOutletContext } from '@/types/projectOutlet';

vi.mock('@/api/client');

const dash = { csrfToken: 'csrf' as string | null, dashboard: { projects: [{ id: 5, name: 'Space' }] } };

vi.mock('@/context/DashboardContext', () => ({
  useDashboard: () => dash,
}));

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual<typeof import('react-router-dom')>('react-router-dom');
  return { ...actual, useOutletContext: vi.fn() };
});

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
        <BaselinesPage />
      </MemoryRouter>
    </ThemeProvider>,
  );
}

describe('BaselinesPage', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    dash.csrfToken = 'csrf';
  });

  it('lists baselines and creates a new one', async () => {
    vi.mocked(apiClient.listBaselines)
      .mockResolvedValueOnce([
        {
          id: 1,
          project_id: 5,
          name: 'PDR',
          description: null,
          created_at: '2024-01-01T00:00:00',
          created_by: 1,
        },
      ])
      .mockResolvedValue([
        {
          id: 1,
          project_id: 5,
          name: 'PDR',
          description: null,
          created_at: '2024-01-01T00:00:00',
          created_by: 1,
        },
        {
          id: 2,
          project_id: 5,
          name: 'CDR',
          description: null,
          created_at: '2024-02-01T00:00:00',
          created_by: 1,
        },
      ]);
    vi.mocked(apiClient.createBaseline).mockResolvedValue({
      id: 2,
      project_id: 5,
      name: 'CDR',
      description: null,
      created_at: '',
      created_by: 1,
    });

    renderPage();
    expect(await screen.findByText('PDR')).toBeInTheDocument();
    await userEvent.type(screen.getByPlaceholderText(/PDR freeze/i), 'CDR');
    await userEvent.click(screen.getByRole('button', { name: /^create$/i }));
    await waitFor(() =>
      expect(apiClient.createBaseline).toHaveBeenCalledWith(5, 'CDR', null, 'csrf'),
    );
    expect(await screen.findByText('CDR')).toBeInTheDocument();
  });

  it('shows CSRF error when token missing', async () => {
    dash.csrfToken = null;
    vi.mocked(apiClient.listBaselines).mockResolvedValue([]);
    renderPage();
    await screen.findByText(/Create baseline/i);
    await userEvent.type(screen.getByPlaceholderText(/PDR freeze/i), 'X');
    await userEvent.click(screen.getByRole('button', { name: /^create$/i }));
    expect(await screen.findByText(/Missing CSRF token/i)).toBeInTheDocument();
  });
});
