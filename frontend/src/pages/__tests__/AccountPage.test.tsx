import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ThemeProvider } from '@/context/ThemeContext';
import AccountPage from '../AccountPage';
import * as apiClient from '@/api/client';

vi.mock('@/api/client');

const refresh = vi.fn();
vi.mock('@/context/DashboardContext', () => ({
  useDashboard: () => ({ csrfToken: 'csrf-test', refresh }),
}));

const me = {
  id: 3,
  username: 'eng_jones',
  name: 'Mike Jones',
  email: 'mike@example.com',
  creation_date: '2026-01-01T00:00:00',
  last_login: '2026-09-01T00:00:00',
  is_admin: false,
  email_verified: true,
};

const serverDeployment = {
  mode: 'server' as const,
  allows_self_registration: false,
  requires_email_verification: false,
  allows_admin_promotion: true,
  assigns_personal_workspace: false,
  allows_self_administered_user_creation: true,
};

function renderPage() {
  return render(
    <ThemeProvider>
      <MemoryRouter>
        <AccountPage />
      </MemoryRouter>
    </ThemeProvider>,
  );
}

describe('AccountPage profile', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(apiClient.getAuthProviders).mockResolvedValue({ password_enabled: true, external: [] });
    vi.mocked(apiClient.getConnectedIdentities).mockResolvedValue({
      password_configured: true,
      identities: [],
    });
    vi.mocked(apiClient.getConnectedApplications).mockResolvedValue([]);
    vi.mocked(apiClient.getMe).mockResolvedValue(me);
    vi.mocked(apiClient.getDeploymentInfo).mockResolvedValue(serverDeployment);
  });

  it('pre-fills the profile and keeps the username read-only', async () => {
    renderPage();
    expect(await screen.findByLabelText('Full name')).toHaveValue('Mike Jones');
    expect(screen.getByLabelText('Email')).toHaveValue('mike@example.com');
    expect(screen.getByLabelText('Username')).toHaveAttribute('readonly');
    expect(screen.queryByLabelText('Current password')).not.toBeInTheDocument();
  });

  it('saves a name change without a password and refreshes the header', async () => {
    vi.mocked(apiClient.updateMyProfile).mockResolvedValue({ ...me, name: 'Michael Jones' });
    renderPage();
    const user = userEvent.setup();
    const name = await screen.findByLabelText('Full name');
    await user.clear(name);
    await user.type(name, 'Michael Jones');
    await user.click(screen.getByRole('button', { name: 'Save profile' }));

    await waitFor(() =>
      expect(apiClient.updateMyProfile).toHaveBeenCalledWith(
        { name: 'Michael Jones', email: 'mike@example.com' },
        'csrf-test',
      ),
    );
    expect(await screen.findByText('Profile updated.')).toBeInTheDocument();
    expect(refresh).toHaveBeenCalled();
  });

  it('asks for the current password when the email changes', async () => {
    vi.mocked(apiClient.updateMyProfile).mockResolvedValue({ ...me, email: 'new@example.com' });
    renderPage();
    const user = userEvent.setup();
    const email = await screen.findByLabelText('Email');
    await user.clear(email);
    await user.type(email, 'new@example.com');

    const password = screen.getByLabelText('Current password');
    expect(password).toBeRequired();
    await user.type(password, 'Orbit!Delta_2026');
    await user.click(screen.getByRole('button', { name: 'Save profile' }));

    await waitFor(() =>
      expect(apiClient.updateMyProfile).toHaveBeenCalledWith(
        { name: 'Mike Jones', email: 'new@example.com', current_password: 'Orbit!Delta_2026' },
        'csrf-test',
      ),
    );
  });

  it('shows backend errors inline', async () => {
    vi.mocked(apiClient.updateMyProfile).mockRejectedValue(new Error('email is already taken'));
    renderPage();
    const user = userEvent.setup();
    const email = await screen.findByLabelText('Email');
    await user.clear(email);
    await user.type(email, 'taken@example.com');
    await user.type(screen.getByLabelText('Current password'), 'Orbit!Delta_2026');
    await user.click(screen.getByRole('button', { name: 'Save profile' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('email is already taken');
    expect(refresh).not.toHaveBeenCalled();
  });

  it('makes the email read-only where emails must be verified', async () => {
    vi.mocked(apiClient.getDeploymentInfo).mockResolvedValue({
      ...serverDeployment,
      mode: 'cloud',
      requires_email_verification: true,
    });
    renderPage();
    await screen.findByLabelText('Full name');
    await waitFor(() => expect(screen.getByLabelText('Email')).toHaveAttribute('readonly'));
    expect(screen.getByText(/Email changes are not available/)).toBeInTheDocument();
  });

  it('does not require a password for SSO-only accounts', async () => {
    vi.mocked(apiClient.getConnectedIdentities).mockResolvedValue({
      password_configured: false,
      identities: [],
    });
    renderPage();
    const user = userEvent.setup();
    const email = await screen.findByLabelText('Email');
    await user.clear(email);
    await user.type(email, 'sso@example.com');
    expect(screen.getByLabelText('Current password')).not.toBeRequired();
    expect(screen.getByText(/Leave empty if you only sign in/)).toBeInTheDocument();
  });
});
