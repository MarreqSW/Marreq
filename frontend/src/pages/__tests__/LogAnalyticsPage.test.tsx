import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ThemeProvider } from '@/context/ThemeContext';
import LogAnalyticsPage, { rangeStart } from '../LogAnalyticsPage';
import type { ProjectOutletContext } from '@/types/projectOutlet';

const mocks = vi.hoisted(() => ({
  listUsersOptional: vi.fn(),
  getAdminLogStats: vi.fn(),
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

const stats = {
  since: '2026-08-28T00:00:00',
  until: '2026-09-26T12:00:00',
  total: 12,
  active_users: 2,
  by_day: [
    { day: '2026-09-24', count: 3 },
    { day: '2026-09-25', count: 0 },
    { day: '2026-09-26', count: 9 },
  ],
  by_action: [
    { action_type: 'UPDATE', count: 8 },
    { action_type: 'CREATE', count: 4 },
  ],
  by_user: [
    { user_id: 1, username: 'alice', count: 10 },
    { user_id: 2, username: 'dr_smith', count: 2 },
  ],
};

function renderPage(entry = '/space-project/admin/logs/analytics') {
  vi.mocked(useOutletContext).mockReturnValue({
    projectId: 5,
    basePath: '/space-project',
    globalSearch: '',
    setGlobalSearch: vi.fn(),
  } satisfies ProjectOutletContext);

  return render(
    <ThemeProvider>
      <MemoryRouter initialEntries={[entry]}>
        <Routes>
          <Route path="/:projectSlug/admin/logs/analytics" element={<LogAnalyticsPage />} />
        </Routes>
      </MemoryRouter>
    </ThemeProvider>,
  );
}

describe('rangeStart', () => {
  it('covers the requested number of UTC days including today', () => {
    const now = new Date(Date.UTC(2026, 8, 26, 23, 30));
    expect(rangeStart(1, now)).toBe('2026-09-26');
    expect(rangeStart(7, now)).toBe('2026-09-20');
    expect(rangeStart(30, now)).toBe('2026-08-28');
  });
});

describe('LogAnalyticsPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.listUsersOptional.mockResolvedValue([{ id: 1, username: 'alice' }]);
    mocks.getAdminLogStats.mockResolvedValue(stats);
  });

  it('shows access denied for non-admins', async () => {
    mocks.listUsersOptional.mockResolvedValue(null);
    renderPage();
    expect(await screen.findByText('Access denied')).toBeInTheDocument();
    expect(mocks.getAdminLogStats).not.toHaveBeenCalled();
  });

  it('renders tiles, the daily chart and rankings for the last 30 days', async () => {
    renderPage();
    expect(await screen.findByText('Events per day')).toBeInTheDocument();
    expect(mocks.getAdminLogStats).toHaveBeenCalledWith({ since: rangeStart(30), top: 10 });

    expect(screen.getByText('Events', { selector: 'p' }).nextElementSibling).toHaveTextContent('12');
    expect(screen.getByText('Average per day').nextElementSibling).toHaveTextContent('4.0');
    expect(screen.getByText('Active users').nextElementSibling).toHaveTextContent('2');
    expect(screen.getByText('Busiest day').nextElementSibling).toHaveTextContent('9');
    expect(screen.getAllByTestId('daily-bar')).toHaveLength(3);

    const actions = screen.getByRole('list', { name: 'Top actions' });
    expect(within(actions).getAllByTestId('ranked-row')).toHaveLength(2);
    expect(within(actions).getByRole('link', { name: 'UPDATE' })).toHaveAttribute(
      'href',
      `/admin/logs?since=${rangeStart(30)}T00%3A00&action_type=UPDATE`,
    );
    const users = screen.getByRole('list', { name: 'Top users' });
    expect(within(users).getByRole('link', { name: 'alice' })).toHaveAttribute(
      'href',
      `/admin/logs?since=${rangeStart(30)}T00%3A00&user_id=1`,
    );
  });

  it('refetches when the range changes', async () => {
    renderPage();
    await screen.findByText('Events per day');
    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: '7 days' }));
    await waitFor(() =>
      expect(mocks.getAdminLogStats).toHaveBeenLastCalledWith({ since: rangeStart(7), top: 10 }),
    );
    expect(screen.getByRole('button', { name: '7 days' })).toHaveAttribute('aria-pressed', 'true');
  });

  it('reads the range from the URL', async () => {
    renderPage('/space-project/admin/logs/analytics?days=90');
    await screen.findByText('Events per day');
    expect(mocks.getAdminLogStats).toHaveBeenCalledWith({ since: rangeStart(90), top: 10 });
  });

  it('shows an empty state without activity', async () => {
    mocks.getAdminLogStats.mockResolvedValue({
      ...stats,
      total: 0,
      active_users: 0,
      by_day: stats.by_day.map((d) => ({ ...d, count: 0 })),
      by_action: [],
      by_user: [],
    });
    renderPage();
    expect(await screen.findByTestId('analytics-empty')).toHaveTextContent('No activity');
    expect(screen.queryByText('Events per day')).not.toBeInTheDocument();
  });

  it('shows API errors', async () => {
    mocks.getAdminLogStats.mockRejectedValue(new Error('range too long'));
    renderPage();
    expect(await screen.findByRole('alert')).toHaveTextContent('range too long');
  });
});
