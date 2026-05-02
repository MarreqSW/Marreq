import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router-dom';
import { ThemeProvider } from '@/context/ThemeContext';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import ResetPasswordPage from '../ResetPasswordPage';
import * as apiClient from '@/api/client';

vi.mock('@/api/client');

function renderResetPage(search = '') {
  return render(
    <ThemeProvider>
      <MemoryRouter initialEntries={[`/reset-password${search}`]}>
        <ResetPasswordPage />
      </MemoryRouter>
    </ThemeProvider>,
  );
}

describe('ResetPasswordPage', () => {
  beforeEach(() => {
    vi.resetAllMocks();
  });

  it('shows "Missing reset token" error immediately when no token in URL', () => {
    renderResetPage();
    expect(screen.getByText(/missing reset token/i)).toBeInTheDocument();
  });

  it('disables inputs and button when no token is present', () => {
    renderResetPage();
    expect(screen.getByLabelText(/new password/i)).toBeDisabled();
    expect(screen.getByLabelText(/confirm password/i)).toBeDisabled();
    expect(screen.getByRole('button', { name: /update password/i })).toBeDisabled();
  });

  it('renders password fields enabled when a token is provided', () => {
    renderResetPage('?token=valid-token');
    expect(screen.getByLabelText(/new password/i)).not.toBeDisabled();
    expect(screen.getByLabelText(/confirm password/i)).not.toBeDisabled();
    expect(screen.getByRole('button', { name: /update password/i })).not.toBeDisabled();
  });

  it('shows error when passwords do not match', async () => {
    renderResetPage('?token=valid-token');
    const user = userEvent.setup();
    await user.type(screen.getByLabelText(/new password/i), 'Password!One_2026');
    await user.type(screen.getByLabelText(/confirm password/i), 'Password!Two_2026');
    await user.click(screen.getByRole('button', { name: /update password/i }));
    expect(screen.getByText(/passwords do not match/i)).toBeInTheDocument();
    expect(apiClient.resetPassword).not.toHaveBeenCalled();
  });

  it('shows password-updated success state after successful reset', async () => {
    vi.mocked(apiClient.resetPassword).mockResolvedValue(undefined);
    renderResetPage('?token=valid-token');

    const user = userEvent.setup();
    await user.type(screen.getByLabelText(/new password/i), 'New!Strong_2026');
    await user.type(screen.getByLabelText(/confirm password/i), 'New!Strong_2026');
    await user.click(screen.getByRole('button', { name: /update password/i }));

    await waitFor(() =>
      expect(screen.getByText(/password updated/i)).toBeInTheDocument(),
    );
    // Form must not be visible after success.
    expect(
      screen.queryByRole('button', { name: /update password/i }),
    ).not.toBeInTheDocument();
  });

  it('shows API error on reset failure', async () => {
    vi.mocked(apiClient.resetPassword).mockRejectedValue(
      new Error('invalid or already-used token'),
    );
    renderResetPage('?token=expired-token');

    const user = userEvent.setup();
    await user.type(screen.getByLabelText(/new password/i), 'New!Strong_2026');
    await user.type(screen.getByLabelText(/confirm password/i), 'New!Strong_2026');
    await user.click(screen.getByRole('button', { name: /update password/i }));

    await waitFor(() =>
      expect(screen.getByText(/invalid or already-used token/i)).toBeInTheDocument(),
    );
  });

  it('renders a link back to sign in', () => {
    renderResetPage('?token=any-token');
    expect(screen.getByRole('link', { name: /back to sign in/i })).toBeInTheDocument();
  });
});
