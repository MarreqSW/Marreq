import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import NoProjectsHome from '../NoProjectsHome';

const navigate = vi.fn();
const logout = vi.fn().mockResolvedValue(undefined);

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual<typeof import('react-router-dom')>('react-router-dom');
  return {
    ...actual,
    useNavigate: () => navigate,
  };
});

vi.mock('@/context/DashboardContext', () => ({
  useDashboard: () => mockDashboard,
}));

let mockDashboard: {
  dashboard: {
    user: { id: number; username: string; name: string; is_admin: boolean };
    projects: Array<{
      id: number;
      name: string;
      project_base_path: string;
      archived?: boolean;
    }>;
    selected_project_id: number | null;
  } | null;
  loading: boolean;
  logout: typeof logout;
};

describe('NoProjectsHome', () => {
  beforeEach(() => {
    navigate.mockReset();
    logout.mockClear();
    mockDashboard = {
      dashboard: null,
      loading: false,
      logout,
    };
  });

  it('offers project and group creation to every authenticated user', () => {
    render(
      <MemoryRouter>
        <NoProjectsHome displayName="Alice" />
      </MemoryRouter>,
    );
    expect(screen.getByRole('link', { name: /new project/i })).toHaveAttribute('href', '/projects/new');
    expect(screen.getByRole('link', { name: /new group/i })).toHaveAttribute('href', '/groups/new');
    expect(screen.getByRole('link', { name: /change password/i })).toHaveAttribute(
      'href',
      '/change-password',
    );
    expect(screen.getByText(/signed in as alice/i)).toBeInTheDocument();
  });

  it('allows a non-admin to start a project and sign out', async () => {
    const user = userEvent.setup();
    render(
      <MemoryRouter>
        <Routes>
          <Route path="/" element={<NoProjectsHome displayName="Bob" />} />
          <Route path="/login" element={<div>login-page</div>} />
        </Routes>
      </MemoryRouter>,
    );
    expect(screen.getByRole('link', { name: /new project/i })).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: /sign out/i }));
    await waitFor(() => expect(logout).toHaveBeenCalled());
    expect(navigate).toHaveBeenCalledWith('/login', { replace: true });
  });
});
