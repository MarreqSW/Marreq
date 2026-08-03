import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ThemeProvider } from '@/context/ThemeContext';
import * as apiClient from '@/api/client';
import GroupCreatePage from '../GroupCreatePage';

vi.mock('@/api/client');

const dash = { csrfToken: 'csrf' as string | null };
const mockNavigate = vi.fn();

vi.mock('@/context/DashboardContext', () => ({
  useDashboard: () => dash,
}));

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual<typeof import('react-router-dom')>('react-router-dom');
  return {
    ...actual,
    useNavigate: () => mockNavigate,
  };
});

function renderPage() {
  return render(
    <ThemeProvider>
      <MemoryRouter>
        <GroupCreatePage />
      </MemoryRouter>
    </ThemeProvider>,
  );
}

describe('GroupCreatePage', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    dash.csrfToken = 'csrf';
  });

  it('shows CSRF error when token missing', async () => {
    dash.csrfToken = null;
    renderPage();
    await userEvent.type(screen.getByPlaceholderText(/Avionics/i), 'Team');
    await userEvent.click(screen.getByRole('button', { name: /create group/i }));
    expect(await screen.findByText(/Missing CSRF token/i)).toBeInTheDocument();
  });

  it('creates group and navigates', async () => {
    vi.mocked(apiClient.createGroup).mockResolvedValue({
      id: 9,
      name: 'Team',
      slug: 'team',
      description: null,
      owner_id: 1,
      created_at: '',
      updated_at: '',
    });
    renderPage();
    await userEvent.type(screen.getByPlaceholderText(/Avionics/i), 'Team');
    await userEvent.click(screen.getByRole('button', { name: /create group/i }));
    await waitFor(() => expect(apiClient.createGroup).toHaveBeenCalled());
    expect(mockNavigate).toHaveBeenCalledWith('/groups/9');
  });
});
