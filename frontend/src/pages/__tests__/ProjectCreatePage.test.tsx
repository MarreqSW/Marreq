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

  it('re-enables submission when project creation fails', async () => {
    const user = userEvent.setup();
    mocks.createProject.mockRejectedValueOnce(new Error('Creation failed'));
    render(
      <MemoryRouter>
        <ProjectCreatePage />
      </MemoryRouter>,
    );

    await user.type(await screen.findByLabelText(/project name/i), 'Autopilot');
    const submit = screen.getByRole('button', { name: /create project/i });
    await user.click(submit);

    expect(await screen.findByRole('alert')).toHaveTextContent('Creation failed');
    expect(mocks.createProject).toHaveBeenCalledTimes(1);
    expect(mocks.refresh).not.toHaveBeenCalled();
    expect(submit).toBeEnabled();
  });

  it('retries post-create handling without creating a duplicate', async () => {
    const user = userEvent.setup();
    mocks.refresh.mockRejectedValueOnce(new Error('Refresh failed'));
    render(
      <MemoryRouter>
        <ProjectCreatePage />
      </MemoryRouter>,
    );

    await user.type(await screen.findByLabelText(/project name/i), 'Autopilot');
    await user.click(screen.getByRole('button', { name: /create project/i }));

    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Project created successfully, but Marreq could not open it.',
    );
    expect(mocks.createProject).toHaveBeenCalledTimes(1);
    expect(mocks.refresh).toHaveBeenCalledTimes(1);
    expect(mocks.navigate).not.toHaveBeenCalled();

    const committedSubmit = screen.getByRole('button', { name: /^project created$/i });
    expect(committedSubmit).toBeDisabled();
    await user.click(committedSubmit);
    expect(mocks.createProject).toHaveBeenCalledTimes(1);

    await user.click(screen.getByRole('button', { name: /retry opening project/i }));

    await waitFor(() =>
      expect(mocks.navigate).toHaveBeenCalledWith('/autopilot/dashboard', { replace: true }),
    );
    expect(mocks.refresh).toHaveBeenCalledTimes(2);
    expect(mocks.createProject).toHaveBeenCalledTimes(1);
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
