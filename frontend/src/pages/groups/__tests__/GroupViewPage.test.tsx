import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ThemeProvider } from '@/context/ThemeContext';
import * as apiClient from '@/api/client';
import GroupViewPage from '../GroupViewPage';

vi.mock('@/api/client');

const mockNavigate = vi.fn();
const refreshDashboard = vi.fn().mockResolvedValue(undefined);

vi.mock('@/context/DashboardContext', () => ({
  useDashboard: () => ({
    csrfToken: 'csrf',
    dashboard: { user: { id: 1, is_admin: false } },
    refresh: refreshDashboard,
  }),
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
      <MemoryRouter initialEntries={['/groups/3']}>
        <Routes>
          <Route path="/groups/:groupId" element={<GroupViewPage />} />
        </Routes>
      </MemoryRouter>
    </ThemeProvider>,
  );
}

describe('GroupViewPage', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(apiClient.getGroup).mockResolvedValue({
      id: 3,
      name: 'Avionics',
      slug: 'avionics',
      description: 'desc',
      owner_id: 1,
      created_at: '',
      updated_at: '',
    });
    vi.mocked(apiClient.listGroupMembers).mockResolvedValue([
      { user_id: 1, role: 1, role_label: 'Owner' },
    ]);
    vi.mocked(apiClient.listGroupProjects).mockResolvedValue([
      {
        id: 10,
        name: 'Space',
        slug: 'space',
        description: null,
        owner_id: 1,
        group_id: 3,
        status: 'active',
        creation_date: null,
        update_date: null,
      },
    ]);
    vi.mocked(apiClient.listUsersOptional).mockResolvedValue([
      {
        id: 1,
        username: 'alice',
        name: 'Alice',
        email: 'a@b.c',
        creation_date: '',
        last_login: '',
        is_admin: false,
      },
    ]);
  });

  it('loads members and projects for an owner', async () => {
    renderPage();
    expect(await screen.findByRole('heading', { name: 'Avionics' })).toBeInTheDocument();
    expect(screen.getByText(/Projects \(1\)/i)).toBeInTheDocument();
    expect(screen.getByRole('link', { name: /Space/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /delete group/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /new project/i })).toBeInTheDocument();
  });

  it('deletes group and navigates away', async () => {
    vi.mocked(apiClient.deleteGroup).mockResolvedValue(undefined);
    vi.spyOn(window, 'confirm').mockReturnValue(true);
    renderPage();
    await screen.findByRole('heading', { name: 'Avionics' });
    await userEvent.click(screen.getByRole('button', { name: /delete group/i }));
    await waitFor(() => expect(apiClient.deleteGroup).toHaveBeenCalledWith(3, 'csrf'));
    expect(mockNavigate).toHaveBeenCalledWith('/groups');
  });

  it('creates a project from the inline form', async () => {
    vi.mocked(apiClient.createProject).mockResolvedValue({
      id: 11,
      name: 'New',
      slug: 'new',
      group_id: 3,
    });
    renderPage();
    await screen.findByRole('heading', { name: 'Avionics' });
    await userEvent.click(screen.getByRole('button', { name: /new project/i }));
    await userEvent.type(screen.getByPlaceholderText(/Flight Control/i), 'New');
    await userEvent.click(screen.getByRole('button', { name: /^create$/i }));
    await waitFor(() =>
      expect(apiClient.createProject).toHaveBeenCalledWith(
        expect.objectContaining({ name: 'New', group_id: 3 }),
        'csrf',
      ),
    );
    expect(mockNavigate).toHaveBeenCalledWith('/avionics/new/dashboard');
  });
});
