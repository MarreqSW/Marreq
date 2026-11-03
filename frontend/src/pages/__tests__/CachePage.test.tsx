import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import CachePage from '../CachePage';

const mocks = vi.hoisted(() => ({
  listUsersOptional: vi.fn(),
  getCsrfToken: vi.fn(),
  getCacheStats: vi.fn(),
  getCacheRecommendations: vi.fn(),
  clearCache: vi.fn(),
  cleanupCache: vi.fn(),
  resetCacheCounters: vi.fn(),
}));

vi.mock('@/api/client', () => mocks);

vi.mock('@/context/DashboardContext', () => ({
  useDashboard: () => ({
    csrfToken: 'csrf-token',
    dashboard: { user: { id: 1, username: 'alice', name: 'Alice', is_admin: true } },
  }),
}));

const stats = {
  total_entries: 10,
  active_entries: 8,
  expired_entries: 2,
  hits: 30,
  misses: 10,
  total_requests: 40,
  // A fraction, not a percentage: the page must show 75.0%, not 0.8%.
  hit_rate: 0.75,
  miss_rate: 0.25,
  average_access_time_ns: 1200,
  total_access_time_ns: 48000,
  last_cleanup: '2026-10-29T08:00:00Z',
  cache_size_bytes: 4096,
};

const advice = {
  recommendations: ['Run a cache cleanup to drop expired entries'],
  priority: 'medium',
  analysis_timestamp: '2026-10-29T08:00:05Z',
};

function renderPage() {
  return render(
    <MemoryRouter initialEntries={['/admin/cache']}>
      <Routes>
        <Route path="/admin/cache" element={<CachePage />} />
      </Routes>
    </MemoryRouter>,
  );
}

describe('CachePage', () => {
  let confirm: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    vi.clearAllMocks();
    confirm = vi.spyOn(window, 'confirm').mockReturnValue(true);
    mocks.listUsersOptional.mockResolvedValue([{ id: 1, username: 'alice' }]);
    mocks.getCsrfToken.mockResolvedValue('fetched-csrf');
    mocks.getCacheStats.mockResolvedValue(stats);
    mocks.getCacheRecommendations.mockResolvedValue(advice);
    mocks.clearCache.mockResolvedValue({ message: 'Cache cleared successfully', timestamp: 't' });
    mocks.cleanupCache.mockResolvedValue({ message: 'done', cleaned_entries: 2, timestamp: 't' });
    mocks.resetCacheCounters.mockResolvedValue({ message: 'reset', timestamp: 't' });
  });

  afterEach(() => confirm.mockRestore());

  it('shows access denied for non-admins without calling the cache API', async () => {
    mocks.listUsersOptional.mockResolvedValue(null);
    renderPage();
    expect(await screen.findByText('Access denied')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Clear all' })).not.toBeInTheDocument();
    expect(mocks.getCacheStats).not.toHaveBeenCalled();
    expect(mocks.getCacheRecommendations).not.toHaveBeenCalled();
  });

  it('shows the entries, the performance figures and the recommendations', async () => {
    renderPage();
    const entries = await screen.findByTestId('cache-entries');
    expect(within(entries).getByText('Total').nextSibling).toHaveTextContent('10');
    expect(within(entries).getByText('Expired').nextSibling).toHaveTextContent('2');
    expect(within(entries).getByText('Size').nextSibling).toHaveTextContent('4 KB');

    const performance = screen.getByTestId('cache-performance');
    expect(within(performance).getByText('Hit rate').nextSibling).toHaveTextContent('75.0%');
    expect(within(performance).getByText('Requests').nextSibling).toHaveTextContent('40');
    expect(within(performance).getByText('Avg access time').nextSibling).toHaveTextContent('1.2 µs');

    expect(screen.getByText('Run a cache cleanup to drop expired entries')).toBeInTheDocument();
    expect(screen.getByTestId('cache-priority')).toHaveTextContent('medium');
  });

  it('still shows the statistics when the recommendations cannot be loaded', async () => {
    mocks.getCacheRecommendations.mockRejectedValue(new Error('boom'));
    renderPage();
    expect(await screen.findByTestId('cache-entries')).toBeInTheDocument();
    expect(screen.getByText('No recommendations.')).toBeInTheDocument();
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  it('shows an error when the statistics cannot be loaded', async () => {
    mocks.getCacheStats.mockRejectedValue(new Error('cache unavailable'));
    renderPage();
    expect(await screen.findByRole('alert')).toHaveTextContent('cache unavailable');
  });

  it('cleans up expired entries without asking and reports how many were removed', async () => {
    renderPage();
    const user = userEvent.setup();
    await user.click(await screen.findByRole('button', { name: 'Clean up expired' }));

    expect(mocks.cleanupCache).toHaveBeenCalledWith('csrf-token');
    expect(confirm).not.toHaveBeenCalled();
    expect(await screen.findByRole('status')).toHaveTextContent('Removed 2 expired entries.');
    expect(mocks.getCacheStats).toHaveBeenCalledTimes(2);
  });

  it('does not clear the cache when the confirmation is declined', async () => {
    confirm.mockReturnValue(false);
    renderPage();
    const user = userEvent.setup();
    await user.click(await screen.findByRole('button', { name: 'Clear all' }));

    expect(confirm).toHaveBeenCalledWith(expect.stringContaining('temporarily slow down'));
    expect(mocks.clearCache).not.toHaveBeenCalled();
    expect(mocks.getCacheStats).toHaveBeenCalledTimes(1);
  });

  it('clears the cache after confirmation and reloads the statistics', async () => {
    renderPage();
    const user = userEvent.setup();
    await user.click(await screen.findByRole('button', { name: 'Clear all' }));

    expect(mocks.clearCache).toHaveBeenCalledWith('csrf-token');
    expect(await screen.findByRole('status')).toHaveTextContent('Cache cleared.');
    await waitFor(() => expect(mocks.getCacheStats).toHaveBeenCalledTimes(2));
  });

  it('resets the counters only after confirmation', async () => {
    confirm.mockReturnValueOnce(false);
    renderPage();
    const user = userEvent.setup();
    const button = await screen.findByRole('button', { name: 'Reset counters' });

    await user.click(button);
    expect(mocks.resetCacheCounters).not.toHaveBeenCalled();

    await user.click(button);
    expect(mocks.resetCacheCounters).toHaveBeenCalledWith('csrf-token');
    expect(await screen.findByRole('status')).toHaveTextContent('Cache counters reset.');
  });

  it('shows the backend error when an action fails and keeps the statistics', async () => {
    mocks.clearCache.mockRejectedValue(new Error('forbidden'));
    renderPage();
    const user = userEvent.setup();
    await user.click(await screen.findByRole('button', { name: 'Clear all' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('forbidden');
    expect(screen.queryByRole('status')).not.toBeInTheDocument();
    expect(screen.getByTestId('cache-entries')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Clear all' })).toBeEnabled();
  });

  it('refreshes the statistics on demand', async () => {
    renderPage();
    const user = userEvent.setup();
    await user.click(await screen.findByRole('button', { name: 'Refresh' }));
    await waitFor(() => expect(mocks.getCacheStats).toHaveBeenCalledTimes(2));
  });
});
