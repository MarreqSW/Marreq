import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router-dom';
import { ThemeProvider } from '@/context/ThemeContext';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import ChangePasswordPage from '../ChangePasswordPage';
import * as apiClient from '@/api/client';

const navigate = vi.fn();

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual<typeof import('react-router-dom')>('react-router-dom');
  return {
    ...actual,
    useNavigate: () => navigate,
  };
});

vi.mock('@/api/client');

vi.mock('@/context/DashboardContext', () => ({
  useDashboard: () => ({ csrfToken: 'csrf-test' }),
}));

function renderPage() {
  return render(
    <ThemeProvider>
      <MemoryRouter>
        <ChangePasswordPage />
      </MemoryRouter>
    </ThemeProvider>,
  );
}

describe('ChangePasswordPage', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    navigate.mockReset();
  });

  it('renders current, new, and confirm password fields', () => {
    renderPage();
    expect(screen.getByLabelText(/current password/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/^new password$/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/confirm new password/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /update password/i })).toBeInTheDocument();
  });

  it('shows error when new passwords do not match', async () => {
    const user = userEvent.setup();
    renderPage();
    await user.type(screen.getByLabelText(/current password/i), 'Voyager!Marble_2026');
    await user.type(screen.getByLabelText(/^new password$/i), 'Password!One_2026');
    await user.type(screen.getByLabelText(/confirm new password/i), 'Password!Two_2026');
    await user.click(screen.getByRole('button', { name: /update password/i }));
    expect(screen.getByText(/passwords do not match/i)).toBeInTheDocument();
    expect(apiClient.changePassword).not.toHaveBeenCalled();
  });

  it('navigates to login after a successful change', async () => {
    vi.mocked(apiClient.changePassword).mockResolvedValue(undefined);
    const user = userEvent.setup();
    renderPage();
    await user.type(screen.getByLabelText(/current password/i), 'Voyager!Marble_2026');
    await user.type(screen.getByLabelText(/^new password$/i), 'Another!Strong_2026');
    await user.type(screen.getByLabelText(/confirm new password/i), 'Another!Strong_2026');
    await user.click(screen.getByRole('button', { name: /update password/i }));

    await waitFor(() =>
      expect(apiClient.changePassword).toHaveBeenCalledWith(
        {
          current_password: 'Voyager!Marble_2026',
          new_password: 'Another!Strong_2026',
          confirm_password: 'Another!Strong_2026',
        },
        'csrf-test',
      ),
    );
    expect(navigate).toHaveBeenCalledWith('/login', { replace: true });
  });

  it('shows API error on failure', async () => {
    vi.mocked(apiClient.changePassword).mockRejectedValue(
      new Error('Current password is incorrect'),
    );
    const user = userEvent.setup();
    renderPage();
    await user.type(screen.getByLabelText(/current password/i), 'wrong');
    await user.type(screen.getByLabelText(/^new password$/i), 'Another!Strong_2026');
    await user.type(screen.getByLabelText(/confirm new password/i), 'Another!Strong_2026');
    await user.click(screen.getByRole('button', { name: /update password/i }));

    await waitFor(() =>
      expect(screen.getByText(/current password is incorrect/i)).toBeInTheDocument(),
    );
    expect(navigate).not.toHaveBeenCalled();
  });
});
