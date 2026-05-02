import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router-dom';
import { ThemeProvider } from '@/context/ThemeContext';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import RegisterPage from '../RegisterPage';
import * as apiClient from '@/api/client';

vi.mock('@/api/client');

function renderRegisterPage() {
  return render(
    <ThemeProvider>
      <MemoryRouter>
        <RegisterPage />
      </MemoryRouter>
    </ThemeProvider>,
  );
}

describe('RegisterPage', () => {
  beforeEach(() => {
    vi.resetAllMocks();
  });

  it('renders username, full name, email, password fields and submit button', () => {
    renderRegisterPage();
    expect(screen.getByLabelText(/username/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/full name/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/email/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/password/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /create account/i })).toBeInTheDocument();
  });

  it('renders a link back to sign in', () => {
    renderRegisterPage();
    expect(screen.getByRole('link', { name: /sign in/i })).toBeInTheDocument();
  });

  it('shows check-your-email message after successful registration', async () => {
    vi.mocked(apiClient.registerAccount).mockResolvedValue(undefined);
    renderRegisterPage();

    const user = userEvent.setup();
    await user.type(screen.getByLabelText(/username/i), 'alice');
    await user.type(screen.getByLabelText(/full name/i), 'Alice Example');
    await user.type(screen.getByLabelText(/email/i), 'alice@example.com');
    await user.type(screen.getByLabelText(/password/i), 'CobaltRiver!Vacuum88');
    await user.click(screen.getByRole('button', { name: /create account/i }));

    await waitFor(() =>
      expect(screen.getByText(/check your email/i)).toBeInTheDocument(),
    );
    // Form should no longer be visible after success.
    expect(screen.queryByRole('button', { name: /create account/i })).not.toBeInTheDocument();
  });

  it('displays error message on registration failure', async () => {
    vi.mocked(apiClient.registerAccount).mockRejectedValue(
      new Error('username already taken'),
    );
    renderRegisterPage();

    const user = userEvent.setup();
    await user.type(screen.getByLabelText(/username/i), 'existing');
    await user.type(screen.getByLabelText(/full name/i), 'Existing User');
    await user.type(screen.getByLabelText(/email/i), 'existing@example.com');
    await user.type(screen.getByLabelText(/password/i), 'CobaltRiver!Vacuum88');
    await user.click(screen.getByRole('button', { name: /create account/i }));

    await waitFor(() =>
      expect(screen.getByText(/username already taken/i)).toBeInTheDocument(),
    );
  });

  it('disables submit button while creating account', async () => {
    vi.mocked(apiClient.registerAccount).mockReturnValue(new Promise(() => {}));
    renderRegisterPage();

    const user = userEvent.setup();
    await user.type(screen.getByLabelText(/username/i), 'alice');
    await user.type(screen.getByLabelText(/full name/i), 'Alice');
    await user.type(screen.getByLabelText(/email/i), 'alice@example.com');
    await user.type(screen.getByLabelText(/password/i), 'CobaltRiver!Vacuum88');
    await user.click(screen.getByRole('button', { name: /create account/i }));

    await waitFor(() =>
      expect(screen.getByRole('button', { name: /creating account/i })).toBeDisabled(),
    );
  });
});
