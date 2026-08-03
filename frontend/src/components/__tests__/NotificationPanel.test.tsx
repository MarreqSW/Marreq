import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ThemeProvider } from '@/context/ThemeContext';
import NotificationPanel from '../NotificationPanel';

const mockNavigate = vi.fn();
const refreshList = vi.fn();
const markRead = vi.fn();
const markAllRead = vi.fn();

const notificationsState = {
  unreadCount: 2,
  notifications: [
    {
      id: 1,
      user_id: 1,
      project_id: 5,
      notification_type: 'comment_added',
      title: 'New comment',
      body: 'Hello',
      entity_type: 'requirement',
      entity_id: 42,
      actor_id: 2,
      read: false,
      emailed: false,
      created_at: '2024-01-01T12:00:00',
    },
    {
      id: 2,
      user_id: 1,
      project_id: 5,
      notification_type: 'review_assigned',
      title: 'Review assigned',
      body: null,
      entity_type: 'verification',
      entity_id: 7,
      actor_id: 2,
      read: true,
      emailed: false,
      created_at: '2024-01-01T10:00:00',
    },
  ],
  loading: false,
  refreshList,
  markRead,
  markAllRead,
};

vi.mock('@/hooks/useNotifications', () => ({
  useNotifications: () => notificationsState,
}));

vi.mock('@/context/DashboardContext', () => ({
  useDashboard: () => ({
    dashboard: {
      projects: [
        {
          id: 5,
          name: 'Space',
          slug: 'space',
          project_base_path: '/alice/space',
        },
      ],
    },
  }),
}));

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual<typeof import('react-router-dom')>('react-router-dom');
  return {
    ...actual,
    useNavigate: () => mockNavigate,
  };
});

function renderPanel() {
  return render(
    <ThemeProvider>
      <MemoryRouter>
        <NotificationPanel />
      </MemoryRouter>
    </ThemeProvider>,
  );
}

describe('NotificationPanel', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    notificationsState.unreadCount = 2;
    notificationsState.loading = false;
    notificationsState.notifications = [
      {
        id: 1,
        user_id: 1,
        project_id: 5,
        notification_type: 'comment_added',
        title: 'New comment',
        body: 'Hello',
        entity_type: 'requirement',
        entity_id: 42,
        actor_id: 2,
        read: false,
        emailed: false,
        created_at: '2024-01-01T12:00:00',
      },
      {
        id: 2,
        user_id: 1,
        project_id: 5,
        notification_type: 'review_assigned',
        title: 'Review assigned',
        body: null,
        entity_type: 'verification',
        entity_id: 7,
        actor_id: 2,
        read: true,
        emailed: false,
        created_at: '2024-01-01T10:00:00',
      },
    ];
  });

  it('shows unread badge and opens list with refresh', async () => {
    renderPanel();
    expect(screen.getByText('2')).toBeInTheDocument();
    await userEvent.click(screen.getByTitle('Notifications'));
    expect(refreshList).toHaveBeenCalled();
    expect(screen.getByText('New comment')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /mark all as read/i })).toBeInTheDocument();
  });

  it('marks all as read', async () => {
    renderPanel();
    await userEvent.click(screen.getByTitle('Notifications'));
    await userEvent.click(screen.getByRole('button', { name: /mark all as read/i }));
    expect(markAllRead).toHaveBeenCalled();
  });

  it('marks one read and navigates to entity path', async () => {
    renderPanel();
    await userEvent.click(screen.getByTitle('Notifications'));
    await userEvent.click(screen.getByText('New comment'));
    expect(markRead).toHaveBeenCalledWith(1);
    expect(mockNavigate).toHaveBeenCalledWith('/alice/space/requirements/42');
  });

  it('closes on Escape', async () => {
    renderPanel();
    await userEvent.click(screen.getByTitle('Notifications'));
    expect(screen.getByText('New comment')).toBeInTheDocument();
    await userEvent.keyboard('{Escape}');
    await waitFor(() => {
      expect(screen.queryByText('New comment')).not.toBeInTheDocument();
    });
  });

  it('shows empty state', async () => {
    notificationsState.unreadCount = 0;
    notificationsState.notifications = [];
    renderPanel();
    await userEvent.click(screen.getByTitle('Notifications'));
    expect(screen.getByText(/No notifications yet/i)).toBeInTheDocument();
  });
});
