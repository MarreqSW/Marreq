import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router-dom';
import { ThemeProvider } from '@/context/ThemeContext';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import ForgotPasswordPage from '../ForgotPasswordPage';
import * as apiClient from '@/api/client';

vi.mock('@/api/client');

function renderForgotPasswordPage() {
  return render(
    <ThemeProvider>
      <MemoryRouter>
        <ForgotPasswordPage />
      </MemoryRouter>
    </ThemeProvider>,
  );
}

describe('ForgotPasswordPage', () => {
  beforeEach(() => {
    vi.resetAllMocks();
  });

  it('renders the email field and submit button', () => {
    renderForgotPasswordPage();
    expect(screen.getByLabelText(/email/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /send reset link/i })).toBeInTheDocument();
  });

  it('renders a link back to sign in', () => {
    renderForgotPasswordPage();
    expect(screen.getByRole('link', { name: /back to sign in/i })).toBeInTheDocument();
  });

  it('shows check-your-email message after successful submission', async () => {
    vi.mocked(apiClient.requestPasswordReset).mockResolvedValue(undefined);
    renderForgotPasswordPage();

    const user = userEvent.setup();
    await user.type(screen.getByLabelText(/email/i), 'alice@example.com');
    await user.click(screen.getByRole('button', { name: /send reset link/i }));

    await waitFor(() =>
      expect(screen.getByText(/check your email/i)).toBeInTheDocument(),
    );
    // Form must not be visible after success.
    expect(
      screen.queryByRole('button', { name: /send reset link/i }),
    ).not.toBeInTheDocument();
  });

  it('displays error message on request failure', async () => {
    vi.mocked(apiClient.requestPasswordReset).mockRejectedValue(
      new Error('Password reset request failed'),
    );
    renderForgotPasswordPage();

    const user = userEvent.setup();
    await user.type(screen.getByLabelText(/email/i), 'alice@example.com');
    await user.click(screen.getByRole('button', { name: /send reset link/i }));

    await waitFor(() =>
      expect(screen.getByText(/password reset request failed/i)).toBeInTheDocument(),
    );
  });

  it('disables submit button while sending', async () => {
    vi.mocked(apiClient.requestPasswordReset).mockReturnValue(new Promise(() => {}));
    renderForgotPasswordPage();

    const user = userEvent.setup();
    await user.type(screen.getByLabelText(/email/i), 'alice@example.com');
    await user.click(screen.getByRole('button', { name: /send reset link/i }));

    await waitFor(() =>
      expect(screen.getByRole('button', { name: /sending/i })).toBeDisabled(),
    );
  });
});
