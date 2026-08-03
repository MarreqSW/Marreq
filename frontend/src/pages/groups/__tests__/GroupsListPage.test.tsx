import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ThemeProvider } from '@/context/ThemeContext';
import * as apiClient from '@/api/client';
import GroupsListPage from '../GroupsListPage';

vi.mock('@/api/client');

vi.mock('@/context/DashboardContext', () => ({
  useDashboard: () => ({ dashboard: { user: { id: 1 } } }),
}));

function renderPage() {
  return render(
    <ThemeProvider>
      <MemoryRouter>
        <GroupsListPage />
      </MemoryRouter>
    </ThemeProvider>,
  );
}

describe('GroupsListPage', () => {
  beforeEach(() => {
    vi.resetAllMocks();
  });

  it('lists groups and filters by name', async () => {
    vi.mocked(apiClient.listGroups).mockResolvedValue([
      {
        id: 1,
        name: 'Avionics',
        slug: 'avionics',
        description: 'a',
        owner_id: 1,
        created_at: '2024-01-01T00:00:00',
        updated_at: '2024-01-01T00:00:00',
      },
      {
        id: 2,
        name: 'Ground',
        slug: 'ground',
        description: null,
        owner_id: 1,
        created_at: '2024-02-01T00:00:00',
        updated_at: '2024-02-01T00:00:00',
      },
    ]);
    renderPage();
    expect(await screen.findByRole('link', { name: 'Avionics' })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Ground' })).toBeInTheDocument();

    await userEvent.type(screen.getByPlaceholderText(/Filter groups/i), 'avio');
    expect(screen.getByRole('link', { name: 'Avionics' })).toBeInTheDocument();
    expect(screen.queryByRole('link', { name: 'Ground' })).not.toBeInTheDocument();
  });

  it('shows empty and error states', async () => {
    vi.mocked(apiClient.listGroups).mockResolvedValue([]);
    const { unmount } = renderPage();
    expect(await screen.findByText(/No groups yet/i)).toBeInTheDocument();
    unmount();

    vi.mocked(apiClient.listGroups).mockRejectedValue(new Error('offline'));
    renderPage();
    expect(await screen.findByText('offline')).toBeInTheDocument();
  });
});
