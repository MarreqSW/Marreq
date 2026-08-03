import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import HomeRedirect from '../HomeRedirect';
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
    }>;
    selected_project_id: number | null;
  } | null;
  loading: boolean;
  logout: typeof logout;
};

describe('HomeRedirect', () => {
  beforeEach(() => {
    navigate.mockReset();
    logout.mockClear();
    mockDashboard = {
      dashboard: null,
      loading: true,
      logout,
    };
  });

  it('shows loading when dashboard is not ready', () => {
    render(
      <MemoryRouter>
        <HomeRedirect />
      </MemoryRouter>,
    );
    expect(screen.getByText(/loading/i)).toBeInTheDocument();
  });

  it('shows NoProjectsHome when there are no projects', () => {
    mockDashboard = {
      dashboard: {
        user: { id: 1, username: 'alice', name: 'Alice', is_admin: false },
        projects: [],
        selected_project_id: null,
      },
      loading: false,
      logout,
    };
    render(
      <MemoryRouter>
        <HomeRedirect />
      </MemoryRouter>,
    );
    expect(screen.getByRole('heading', { name: /no projects available/i })).toBeInTheDocument();
    expect(screen.getByText(/not a member of any project/i)).toBeInTheDocument();
  });

  it('navigates to the selected project dashboard when projects exist', async () => {
    mockDashboard = {
      dashboard: {
        user: { id: 1, username: 'alice', name: 'Alice', is_admin: true },
        projects: [
          { id: 10, name: 'Space', project_base_path: '/alice/space-project' },
          { id: 11, name: 'Other', project_base_path: '/alice/other' },
        ],
        selected_project_id: 10,
      },
      loading: false,
      logout,
    };
    render(
      <MemoryRouter>
        <HomeRedirect />
      </MemoryRouter>,
    );
    await waitFor(() =>
      expect(navigate).toHaveBeenCalledWith('/alice/space-project/dashboard', { replace: true }),
    );
    expect(screen.getByText(/opening project/i)).toBeInTheDocument();
  });
});

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

  it('offers create project for admins', () => {
    render(
      <MemoryRouter>
        <NoProjectsHome isAdmin displayName="Alice" />
      </MemoryRouter>,
    );
    expect(screen.getByRole('link', { name: /create project/i })).toHaveAttribute(
      'href',
      '/new_project',
    );
    expect(screen.getByText(/signed in as alice/i)).toBeInTheDocument();
  });

  it('hides create project for non-admins and signs out', async () => {
    const user = userEvent.setup();
    render(
      <MemoryRouter>
        <Routes>
          <Route path="/" element={<NoProjectsHome isAdmin={false} displayName="Bob" />} />
          <Route path="/login" element={<div>login-page</div>} />
        </Routes>
      </MemoryRouter>,
    );
    expect(screen.queryByRole('link', { name: /create project/i })).not.toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: /sign out/i }));
    await waitFor(() => expect(logout).toHaveBeenCalled());
    expect(navigate).toHaveBeenCalledWith('/login', { replace: true });
  });
});
