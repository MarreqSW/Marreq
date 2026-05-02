import { render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { ThemeProvider } from '@/context/ThemeContext';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import VerifyEmailPage from '../VerifyEmailPage';
import * as apiClient from '@/api/client';

vi.mock('@/api/client');

function renderVerifyPage(search = '') {
  return render(
    <ThemeProvider>
      <MemoryRouter initialEntries={[`/verify-email${search}`]}>
        <VerifyEmailPage />
      </MemoryRouter>
    </ThemeProvider>,
  );
}

describe('VerifyEmailPage', () => {
  beforeEach(() => {
    vi.resetAllMocks();
  });

  it('shows "Missing verification token" error immediately when no token in URL', () => {
    renderVerifyPage();
    expect(screen.getByText(/missing verification token/i)).toBeInTheDocument();
    expect(apiClient.verifyEmail).not.toHaveBeenCalled();
  });

  it('renders a link back to sign in', () => {
    renderVerifyPage();
    expect(screen.getByRole('link', { name: /back to sign in/i })).toBeInTheDocument();
  });

  it('shows loading state initially when a token is present', () => {
    vi.mocked(apiClient.verifyEmail).mockReturnValue(new Promise(() => {}));
    renderVerifyPage('?token=pending-token');
    expect(screen.getByText(/verifying your email/i)).toBeInTheDocument();
  });

  it('shows success message after verifyEmail resolves', async () => {
    vi.mocked(apiClient.verifyEmail).mockResolvedValue(undefined);
    renderVerifyPage('?token=valid-token');

    await waitFor(() =>
      expect(
        screen.getByText(/your email address has been verified/i),
      ).toBeInTheDocument(),
    );
  });

  it('shows error message when verifyEmail rejects', async () => {
    vi.mocked(apiClient.verifyEmail).mockRejectedValue(
      new Error('invalid or already-used token'),
    );
    renderVerifyPage('?token=bad-token');

    await waitFor(() =>
      expect(screen.getByText(/invalid or already-used token/i)).toBeInTheDocument(),
    );
  });

  it('calls verifyEmail with the token from the URL', async () => {
    vi.mocked(apiClient.verifyEmail).mockResolvedValue(undefined);
    renderVerifyPage('?token=abc-123-xyz');

    await waitFor(() => expect(apiClient.verifyEmail).toHaveBeenCalledWith('abc-123-xyz'));
  });
});
