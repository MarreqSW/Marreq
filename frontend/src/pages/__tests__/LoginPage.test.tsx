import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router-dom';
import { ThemeProvider } from '@/context/ThemeContext';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import LoginPage from '../LoginPage';
import * as apiClient from '@/api/client';

vi.mock('@/api/client');

function renderLoginPage() {
  return render(
    <ThemeProvider>
      <MemoryRouter>
        <LoginPage />
      </MemoryRouter>
    </ThemeProvider>,
  );
}

const serverDeployment = {
  mode: 'server' as const,
  allows_self_registration: false,
  requires_email_verification: false,
  allows_admin_promotion: true,
  assigns_personal_workspace: false,
  allows_self_administered_user_creation: true,
};

const cloudDeployment = {
  mode: 'cloud' as const,
  allows_self_registration: true,
  requires_email_verification: true,
  allows_admin_promotion: false,
  assigns_personal_workspace: true,
  allows_self_administered_user_creation: false,
};

describe('LoginPage', () => {
  beforeEach(() => {
    vi.resetAllMocks();
  });

  it('renders the username and password fields and sign-in button', async () => {
    vi.mocked(apiClient.getDeploymentInfo).mockResolvedValue(serverDeployment);
    renderLoginPage();
    expect(screen.getByLabelText(/username/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/password/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /sign in/i })).toBeInTheDocument();
  });

  it('hides self-service links in server mode', async () => {
    vi.mocked(apiClient.getDeploymentInfo).mockResolvedValue(serverDeployment);
    renderLoginPage();
    await waitFor(() => expect(apiClient.getDeploymentInfo).toHaveBeenCalled());
    expect(screen.queryByRole('link', { name: /create an account/i })).not.toBeInTheDocument();
    expect(screen.queryByRole('link', { name: /forgot your password/i })).not.toBeInTheDocument();
  });

  it('shows register and forgot-password links in cloud mode', async () => {
    vi.mocked(apiClient.getDeploymentInfo).mockResolvedValue(cloudDeployment);
    renderLoginPage();
    await waitFor(() =>
      expect(screen.getByRole('link', { name: /create an account/i })).toBeInTheDocument(),
    );
    expect(screen.getByRole('link', { name: /forgot your password/i })).toBeInTheDocument();
  });

  it('hides self-service links when deployment info fails to load', async () => {
    vi.mocked(apiClient.getDeploymentInfo).mockRejectedValue(new Error('offline'));
    renderLoginPage();
    await waitFor(() => expect(apiClient.getDeploymentInfo).toHaveBeenCalled());
    expect(screen.queryByRole('link', { name: /create an account/i })).not.toBeInTheDocument();
  });

  it('displays error message on login failure', async () => {
    vi.mocked(apiClient.getDeploymentInfo).mockResolvedValue(serverDeployment);
    vi.mocked(apiClient.getCsrfToken).mockResolvedValue('test-csrf');
    vi.mocked(apiClient.loginJson).mockRejectedValue(new Error('Invalid username or password'));
    renderLoginPage();

    const user = userEvent.setup();
    await user.type(screen.getByLabelText(/username/i), 'alice');
    await user.type(screen.getByLabelText(/password/i), 'wrongpass');
    await user.click(screen.getByRole('button', { name: /sign in/i }));

    await waitFor(() =>
      expect(screen.getByText(/invalid username or password/i)).toBeInTheDocument(),
    );
  });

  it('disables submit button while signing in', async () => {
    vi.mocked(apiClient.getDeploymentInfo).mockResolvedValue(serverDeployment);
    vi.mocked(apiClient.getCsrfToken).mockResolvedValue('test-csrf');
    // Never resolves — keeps submitting state active.
    vi.mocked(apiClient.loginJson).mockReturnValue(new Promise(() => {}));
    renderLoginPage();

    const user = userEvent.setup();
    await user.type(screen.getByLabelText(/username/i), 'alice');
    await user.type(screen.getByLabelText(/password/i), 'password');
    await user.click(screen.getByRole('button', { name: /sign in/i }));

    await waitFor(() =>
      expect(screen.getByRole('button', { name: /signing in/i })).toBeDisabled(),
    );
  });
});
