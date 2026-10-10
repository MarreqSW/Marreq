import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes, useLocation } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as apiClient from '@/api/client';
import type { AttentionResponse } from '@/api/types';
import { recordVisit, setVisitDetail } from '@/utils/recentPages';
import StartPage, { attentionSummary, shortAge } from '../StartPage';

vi.mock('@/api/client');
vi.mock('@/components/NotificationPanel', () => ({ default: () => null }));
vi.mock('@/context/ThemeContext', () => ({
  useTheme: () => ({ preference: 'light', setPreference: vi.fn() }),
}));

const projects = [
  { id: 5, name: 'Space Project', slug: 'space-project', project_base_path: '/space-project', group_id: 1, group_name: 'Satellite Team', group_slug: 'sat', archived: false, role_label: 'Admin' },
  { id: 6, name: 'Rover', slug: 'rover', project_base_path: '/rover', group_id: null, group_name: null, group_slug: null, archived: false },
  { id: 7, name: 'Old Probe', slug: 'old-probe', project_base_path: '/old-probe', group_id: null, group_name: null, group_slug: null, archived: true },
];

let dashboard: { user: unknown; projects: typeof projects } | null;
vi.mock('@/context/DashboardContext', () => ({
  useDashboard: () => ({ dashboard, loading: false, csrfToken: 'csrf', logout: vi.fn() }),
}));

const now = new Date();
const hoursAgo = (h: number) => new Date(now.getTime() - h * 3_600_000).toISOString().replace('Z', '');

const attention: AttentionResponse = {
  items: [
    { kind: 'approval', project_id: 5, title: 'REQ-1 is waiting for your approval', requirement_id: 11, reference_code: 'REQ-1', count: null, notification_id: null, notification_type: null, at: hoursAgo(1) },
    { kind: 'notification', project_id: 6, title: 'Bob commented on NAV-7', requirement_id: 70, reference_code: null, count: null, notification_id: 99, notification_type: 'comment_added', at: hoursAgo(3) },
    { kind: 'suspect', project_id: 6, title: '3 suspect links to review', requirement_id: null, reference_code: null, count: 3, notification_id: null, notification_type: null, at: hoursAgo(5) },
    { kind: 'review', project_id: 5, title: '2 draft requirements to review', requirement_id: null, reference_code: null, count: 2, notification_id: null, notification_type: null, at: hoursAgo(30) },
    { kind: 'approval', project_id: 5, title: 'REQ-2 is waiting for your approval', requirement_id: 12, reference_code: 'REQ-2', count: null, notification_id: null, notification_type: null, at: hoursAgo(50) },
  ],
  approvals: 2,
  reviews: 2,
  suspect_links: 3,
  notifications: 1,
};

function Where() {
  return <p data-testid="where">{useLocation().pathname + useLocation().search}</p>;
}

function renderStart() {
  return render(
    <MemoryRouter initialEntries={['/']}>
      <Routes>
        <Route path="/" element={<StartPage />} />
        <Route path="*" element={<Where />} />
      </Routes>
    </MemoryRouter>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  localStorage.clear();
  dashboard = { user: { id: 1, username: 'alice', name: 'Alice Johnson', is_admin: false }, projects };
  vi.mocked(apiClient.getAttention).mockResolvedValue(attention);
  vi.mocked(apiClient.markNotificationRead).mockResolvedValue(undefined);
  vi.mocked(apiClient.searchRequirements).mockResolvedValue([]);
});

describe('StartPage', () => {
  it('greets the user, summarises and lists what needs attention', async () => {
    renderStart();
    expect(screen.getByRole('heading', { name: 'Where to, Alice?' })).toBeInTheDocument();
    expect(
      await screen.findByText(
        '2 approvals, 2 drafts to review, 3 suspect links and 1 notification are waiting for you.',
      ),
    ).toBeInTheDocument();
    const list = screen.getByRole('heading', { name: /needs your attention/i }).closest('section')!;
    const links = within(list).getAllByRole('link');
    expect(links).toHaveLength(4);
    expect(links[0]).toHaveAttribute('href', '/space-project/requirements/11');
    expect(links[2]).toHaveAttribute('href', '/rover/traceability?view=matrix&mx_suspect=1');
    expect(links[3]).toHaveAttribute('href', '/space-project/requirements?approval=draft');

    await userEvent.click(within(list).getByRole('button', { name: 'Show all (5)' }));
    expect(within(list).getAllByRole('link')).toHaveLength(5);
  });

  it('marks a notification read when it is opened', async () => {
    renderStart();
    await userEvent.click(await screen.findByRole('link', { name: /bob commented on nav-7/i }));
    expect(apiClient.markNotificationRead).toHaveBeenCalledWith(99, 'csrf');
    expect(screen.getByTestId('where')).toHaveTextContent('/rover/requirements/70');
  });

  it('says so when nothing needs attention, or when it cannot load', async () => {
    vi.mocked(apiClient.getAttention).mockResolvedValueOnce({
      items: [],
      approvals: 0,
      reviews: 0,
      suspect_links: 0,
      notifications: 0,
    });
    const { unmount } = renderStart();
    expect(await screen.findByText('You are all caught up.')).toBeInTheDocument();
    expect(screen.getByText('Nothing needs your attention right now.')).toBeInTheDocument();
    unmount();

    vi.mocked(apiClient.getAttention).mockRejectedValueOnce(new Error('down'));
    renderStart();
    expect(await screen.findByText('Could not load what needs your attention.')).toBeInTheDocument();
  });

  it('lists recent pages, or the projects when there are none', async () => {
    const { unmount } = renderStart();
    expect(screen.getByRole('heading', { name: 'Your projects' })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: /space project satellite team admin/i })).toHaveAttribute(
      'href',
      '/space-project/dashboard',
    );
    unmount();

    recordVisit(6, '/rover/traceability?view=matrix', '/traceability', Date.now() - 60_000);
    recordVisit(5, '/space-project/requirements/11', '/requirements/11');
    setVisitDetail(5, '/space-project/requirements/11', 'REQ-1');
    recordVisit(99, '/gone/dashboard', '/dashboard'); // no longer a member
    renderStart();
    const recent = screen.getByRole('heading', { name: 'Recent' }).closest('section')!;
    const links = within(recent).getAllByRole('link');
    expect(links.map((l) => l.getAttribute('href'))).toEqual([
      '/space-project/requirements/11',
      '/rover/traceability?view=matrix',
    ]);
    expect(links[0]).toHaveTextContent('Requirements › REQ-1');
    await screen.findByText(/are waiting for you\./);
  });

  it('shows all projects and the archived ones on request', async () => {
    renderStart();
    await userEvent.click(screen.getByRole('button', { name: 'All projects (2)' }));
    const all = screen.getByRole('region', { name: 'All projects' });
    expect(within(all).getAllByRole('link')).toHaveLength(2);
    await userEvent.click(screen.getByRole('button', { name: 'Archived (1)' }));
    const archived = screen.getByRole('region', { name: 'Archived projects' });
    expect(within(archived).getByRole('link', { name: /old probe/i })).toHaveAttribute(
      'href',
      '/old-probe/dashboard',
    );
    expect(screen.getByRole('link', { name: /new project/i })).toHaveAttribute('href', '/projects/new');
    await screen.findByText(/are waiting for you\./);
  });

  it('shows the no-projects page when the user has none', () => {
    dashboard = { user: { id: 1, username: 'alice', name: 'Alice', is_admin: false }, projects: [] };
    renderStart();
    expect(screen.getByText("You don't have any projects yet")).toBeInTheDocument();
    expect(apiClient.getAttention).not.toHaveBeenCalled();
  });

  it('offers Administration in the user menu to admins only', async () => {
    renderStart();
    await userEvent.click(screen.getByRole('button', { name: 'User menu' }));
    expect(screen.queryByRole('menuitem', { name: 'Administration' })).not.toBeInTheDocument();
    await waitFor(() => expect(apiClient.getAttention).toHaveBeenCalled());
  });
});

describe('attentionSummary and shortAge', () => {
  it('reads naturally', () => {
    const base = { items: [], approvals: 0, reviews: 0, suspect_links: 0, notifications: 0 };
    expect(attentionSummary({ ...base, approvals: 1 })).toBe('1 approval is waiting for you.');
    expect(attentionSummary({ ...base, reviews: 2, suspect_links: 1 })).toBe(
      '2 drafts to review and 1 suspect link are waiting for you.',
    );
  });

  it('formats ages compactly', () => {
    const t = Date.UTC(2026, 9, 10, 12, 0, 0);
    expect(shortAge('2026-10-10T11:30:00', t)).toBe('30 min');
    expect(shortAge('2026-10-10T07:00:00', t)).toBe('5 h');
    expect(shortAge('2026-10-07T12:00:00', t)).toBe('3 d');
    expect(shortAge('2026-01-10T12:00:00', t)).toBe('9 mo');
    expect(shortAge('2023-10-10T12:00:00', t)).toBe('3 y');
    expect(shortAge(null, t)).toBe('');
  });
});
