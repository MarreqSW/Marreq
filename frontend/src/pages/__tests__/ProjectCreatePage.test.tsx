import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import ProjectCreatePage from '../ProjectCreatePage';

const mocks = vi.hoisted(() => ({
  createProject: vi.fn(),
  listCreatableGroups: vi.fn(),
  navigate: vi.fn(),
  refresh: vi.fn(),
}));

vi.mock('@/api/client', () => ({
  createProject: mocks.createProject,
  listCreatableGroups: mocks.listCreatableGroups,
}));

vi.mock('@/context/DashboardContext', () => ({
  useDashboard: () => ({
    csrfToken: 'csrf-token',
    dashboard: {
      user: { id: 1, username: 'alice', name: 'Alice', is_admin: false },
      projects: [],
    },
    refresh: mocks.refresh,
  }),
}));

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual<typeof import('react-router-dom')>('react-router-dom');
  return { ...actual, useNavigate: () => mocks.navigate };
});

const flightSystems = {
  id: 10,
  name: 'Flight Systems',
  slug: 'flight-systems',
  description: null,
  owner_id: 1,
  created_at: '2026-01-01T00:00:00',
  updated_at: '2026-01-01T00:00:00',
};

describe('ProjectCreatePage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.listCreatableGroups.mockResolvedValue([flightSystems]);
    mocks.refresh.mockResolvedValue(undefined);
    mocks.createProject.mockResolvedValue({
      id: 42,
      name: 'Autopilot',
      slug: 'autopilot',
      group_id: null,
      project_base_path: '/autopilot',
    });
  });

  it('shows the personal namespace and only groups returned as creatable', async () => {
    render(
      <MemoryRouter>
        <ProjectCreatePage />
      </MemoryRouter>,
    );

    const namespace = await screen.findByLabelText(/namespace/i);
    expect(namespace).toHaveTextContent('alice — Personal');
    expect(namespace).toHaveTextContent('flight-systems — Group');
    expect(namespace).not.toHaveTextContent('view-only-team');
  });

  it('creates a personal project with a null group and uses the canonical path', async () => {
    const user = userEvent.setup();
    render(
      <MemoryRouter>
        <ProjectCreatePage />
      </MemoryRouter>,
    );

    await user.type(await screen.findByLabelText(/project name/i), 'Autopilot');
    await user.click(screen.getByRole('button', { name: /create project/i }));

    await waitFor(() =>
      expect(mocks.createProject).toHaveBeenCalledWith(
        { name: 'Autopilot', description: null, group_id: null },
        'csrf-token',
      ),
    );
    expect(mocks.refresh).toHaveBeenCalled();
    expect(mocks.navigate).toHaveBeenCalledWith('/autopilot/dashboard', { replace: true });
  });

  it('submits the selected group id', async () => {
    const user = userEvent.setup();
    mocks.createProject.mockResolvedValue({
      id: 43,
      name: 'Autopilot',
      slug: 'autopilot',
      group_id: 10,
      project_base_path: '/autopilot',
    });
    render(
      <MemoryRouter>
        <ProjectCreatePage />
      </MemoryRouter>,
    );

    await user.selectOptions(await screen.findByLabelText(/namespace/i), 'group:10');
    await user.type(screen.getByLabelText(/project name/i), 'Autopilot');
    await user.click(screen.getByRole('button', { name: /create project/i }));

    await waitFor(() =>
      expect(mocks.createProject).toHaveBeenCalledWith(
        { name: 'Autopilot', description: null, group_id: 10 },
        'csrf-token',
      ),
    );
  });
});
