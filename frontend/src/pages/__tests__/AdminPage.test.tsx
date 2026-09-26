import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ThemeProvider } from '@/context/ThemeContext';
import AdminPage from '../AdminPage';
import type { ProjectOutletContext } from '@/types/projectOutlet';

const mocks = vi.hoisted(() => ({
  listUsersOptional: vi.fn(),
  getDeploymentInfo: vi.fn(),
  getCsrfToken: vi.fn(),
  createUser: vi.fn(),
  updateUser: vi.fn(),
  setUserPassword: vi.fn(),
  deleteUser: vi.fn(),
}));

vi.mock('@/api/client', () => mocks);

vi.mock('@/context/DashboardContext', () => ({
  useDashboard: () => ({
    csrfToken: 'csrf-token',
    dashboard: {
      user: { id: 1, username: 'alice', name: 'Alice', is_admin: true },
      projects: [{ id: 5, name: 'Space Project' }],
    },
  }),
}));

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual<typeof import('react-router-dom')>('react-router-dom');
  return { ...actual, useOutletContext: vi.fn() };
});

import { useOutletContext } from 'react-router-dom';

const users = [
  { id: 1, username: 'alice', name: 'Alice', email: 'alice@marreq.com', is_admin: true },
  { id: 2, username: 'bob', name: 'Bob', email: 'bob@marreq.com', is_admin: false },
];

const serverDeployment = {
  mode: 'server',
  allows_self_registration: false,
  requires_email_verification: false,
  allows_admin_promotion: true,
  assigns_personal_workspace: false,
  allows_self_administered_user_creation: true,
};

const cloudDeployment = {
  ...serverDeployment,
  mode: 'cloud',
  allows_self_registration: true,
  allows_admin_promotion: false,
  allows_self_administered_user_creation: false,
};

function renderPage() {
  vi.mocked(useOutletContext).mockReturnValue({
    projectId: 5,
    basePath: '/space-project',
    globalSearch: '',
    setGlobalSearch: vi.fn(),
  } satisfies ProjectOutletContext);

  return render(
    <ThemeProvider>
      <MemoryRouter initialEntries={['/space-project/admin']}>
        <Routes>
          <Route path="/:projectSlug/admin" element={<AdminPage />} />
        </Routes>
      </MemoryRouter>
    </ThemeProvider>,
  );
}

function row(username: string) {
  return screen.getByText(username).closest('tr') as HTMLElement;
}

describe('AdminPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    window.confirm = vi.fn(() => true);
    mocks.listUsersOptional.mockResolvedValue(users);
    mocks.getDeploymentInfo.mockResolvedValue(serverDeployment);
  });

  it('shows access denied when the user list is forbidden', async () => {
    mocks.listUsersOptional.mockResolvedValue(null);
    renderPage();
    expect(await screen.findByText('Access denied')).toBeInTheDocument();
  });

  it('lists users with row actions', async () => {
    renderPage();
    expect(await screen.findByText('bob')).toBeInTheDocument();
    const bob = within(row('bob'));
    expect(bob.getByRole('button', { name: 'Edit' })).toBeInTheDocument();
    expect(bob.getByRole('button', { name: 'Set password' })).toBeInTheDocument();
    expect(bob.getByRole('button', { name: 'Delete' })).toBeEnabled();
    expect(within(row('alice')).getByRole('button', { name: 'Delete' })).toBeDisabled();
  });

  it('creates a user and reloads the list', async () => {
    mocks.createUser.mockResolvedValue({ id: 3 });
    renderPage();
    const user = userEvent.setup();
    await user.click(await screen.findByRole('button', { name: 'New user' }));

    const dialog = screen.getByRole('dialog');
    await user.type(within(dialog).getByLabelText('Username'), 'guest');
    await user.type(within(dialog).getByLabelText('Full name'), 'Guest');
    await user.type(within(dialog).getByLabelText('Email'), 'guest@demo.local');
    await user.type(within(dialog).getByLabelText('Password'), 'Orbit!Delta_2026');
    await user.type(within(dialog).getByLabelText('Confirm password'), 'Orbit!Delta_2026');
    await user.click(within(dialog).getByRole('button', { name: 'Create user' }));

    await waitFor(() =>
      expect(mocks.createUser).toHaveBeenCalledWith(
        {
          username: 'guest',
          name: 'Guest',
          email: 'guest@demo.local',
          is_admin: false,
          password: 'Orbit!Delta_2026',
        },
        'csrf-token',
      ),
    );
    expect(await screen.findByText('User created.')).toBeInTheDocument();
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(mocks.listUsersOptional).toHaveBeenCalledTimes(2);
  });

  it('shows password policy errors inside the form', async () => {
    mocks.createUser.mockRejectedValue(new Error('Password is too common. Choose a more unique password'));
    renderPage();
    const user = userEvent.setup();
    await user.click(await screen.findByRole('button', { name: 'New user' }));

    const dialog = screen.getByRole('dialog');
    await user.type(within(dialog).getByLabelText('Username'), 'guest');
    await user.type(within(dialog).getByLabelText('Full name'), 'Guest');
    await user.type(within(dialog).getByLabelText('Email'), 'guest@demo.local');
    await user.type(within(dialog).getByLabelText('Password'), 'password1');
    await user.type(within(dialog).getByLabelText('Confirm password'), 'password1');
    await user.click(within(dialog).getByRole('button', { name: 'Create user' }));

    expect(await within(dialog).findByRole('alert')).toHaveTextContent('Password is too common');
    expect(screen.getByRole('dialog')).toBeInTheDocument();
  });

  it('hides user creation and locks the admin flag when users self-register', async () => {
    mocks.getDeploymentInfo.mockResolvedValue(cloudDeployment);
    renderPage();
    await screen.findByText('bob');
    await waitFor(() =>
      expect(screen.getByText(/Users self-register in this deployment/)).toBeInTheDocument(),
    );
    expect(screen.queryByRole('button', { name: 'New user' })).not.toBeInTheDocument();

    const user = userEvent.setup();
    await user.click(within(row('bob')).getByRole('button', { name: 'Edit' }));
    const dialog = screen.getByRole('dialog');
    expect(within(dialog).getByLabelText('Site administrator')).toBeDisabled();
    expect(within(dialog).getByText(/Admin promotion is disabled/)).toBeInTheDocument();
  });

  it('edits a user', async () => {
    mocks.updateUser.mockResolvedValue({ ...users[1], name: 'Robert' });
    renderPage();
    const user = userEvent.setup();
    await screen.findByText('bob');
    await user.click(within(row('bob')).getByRole('button', { name: 'Edit' }));

    const dialog = screen.getByRole('dialog');
    const name = within(dialog).getByLabelText('Full name');
    await user.clear(name);
    await user.type(name, 'Robert');
    await user.click(within(dialog).getByLabelText('Site administrator'));
    await user.click(within(dialog).getByRole('button', { name: 'Save' }));

    await waitFor(() =>
      expect(mocks.updateUser).toHaveBeenCalledWith(
        2,
        { username: 'bob', name: 'Robert', email: 'bob@marreq.com', is_admin: true },
        'csrf-token',
      ),
    );
    expect(await screen.findByText('Saved bob.')).toBeInTheDocument();
  });

  it('sets a password', async () => {
    mocks.setUserPassword.mockResolvedValue(undefined);
    renderPage();
    const user = userEvent.setup();
    await screen.findByText('bob');
    await user.click(within(row('bob')).getByRole('button', { name: 'Set password' }));

    const dialog = screen.getByRole('dialog');
    await user.type(within(dialog).getByLabelText('New password'), 'Patata!Orbit_314');
    await user.type(within(dialog).getByLabelText('Confirm new password'), 'Patata!Orbit_314');
    await user.click(within(dialog).getByRole('button', { name: 'Set password' }));

    await waitFor(() =>
      expect(mocks.setUserPassword).toHaveBeenCalledWith(
        2,
        'Patata!Orbit_314',
        'Patata!Orbit_314',
        'csrf-token',
      ),
    );
    expect(await screen.findByText('Password set for bob.')).toBeInTheDocument();
  });

  it('deletes a user after confirmation and shows API errors', async () => {
    mocks.deleteUser.mockResolvedValueOnce(undefined);
    renderPage();
    const user = userEvent.setup();
    await screen.findByText('bob');

    await user.click(within(row('bob')).getByRole('button', { name: 'Delete' }));
    expect(window.confirm).toHaveBeenCalled();
    await waitFor(() => expect(mocks.deleteUser).toHaveBeenCalledWith(2, 'csrf-token'));
    expect(await screen.findByText('Deleted bob.')).toBeInTheDocument();

    mocks.deleteUser.mockRejectedValueOnce(new Error('User still owns or authored records'));
    await user.click(within(row('bob')).getByRole('button', { name: 'Delete' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('User still owns or authored records');
  });

  it('does not delete when the confirmation is cancelled', async () => {
    window.confirm = vi.fn(() => false);
    renderPage();
    const user = userEvent.setup();
    await screen.findByText('bob');
    await user.click(within(row('bob')).getByRole('button', { name: 'Delete' }));
    expect(mocks.deleteUser).not.toHaveBeenCalled();
  });
});
