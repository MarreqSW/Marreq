import { act, render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes, useLocation } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as apiClient from '@/api/client';
import { readRecentPages } from '@/utils/recentPages';
import ProjectLayout from '../ProjectLayout';

const dashboard = vi.hoisted(() => ({
  value: {
    projects: [
      { id: 5, name: 'Space Project', slug: 'space-project', project_base_path: '/space-project' },
      { id: 6, name: 'Rover', slug: 'rover', project_base_path: '/rover' },
      {
        id: 7,
        name: 'Old Probe',
        slug: 'old-probe',
        project_base_path: '/old-probe',
        archived: true,
        owner_id: 2,
      },
    ],
    user: { id: 1, username: 'alice', name: 'Alice Johnson', is_admin: true },
  },
}));

const refresh = vi.hoisted(() => vi.fn());
vi.mock('@/context/DashboardContext', () => ({
  useDashboard: () => ({
    dashboard: dashboard.value,
    csrfToken: 'csrf',
    setSelectedProjectId: vi.fn(),
    refresh,
    logout: vi.fn(),
  }),
}));
vi.mock('@/context/ThemeContext', () => ({
  useTheme: () => ({ preference: 'light', setPreference: vi.fn() }),
}));
vi.mock('@/components/NotificationPanel', () => ({ default: () => null }));
vi.mock('@/hooks/useBuildInfo', () => ({ useBuildInfo: () => ({ build: null, error: false }) }));
vi.mock('@/api/client', () => ({ getProjectFromPath: vi.fn(), setProjectArchived: vi.fn() }));

function Where() {
  return <output data-testid="where">{useLocation().pathname}</output>;
}

function renderAt(path = '/space-project/dashboard') {
  return render(
    <MemoryRouter initialEntries={[path]}>
      <Routes>
        <Route path="/:projectSlug" element={<ProjectLayout />}>
          <Route path="*" element={<Where />} />
        </Route>
        <Route path="/admin" element={<p>admin area</p>} />
      </Routes>
    </MemoryRouter>,
  );
}

beforeEach(() => {
  localStorage.clear();
  dashboard.value.user.is_admin = true;
});

describe('ProjectLayout navigation', () => {
  it('shows only daily work plus Project settings and Help', () => {
    renderAt();
    const sidebar = screen.getByRole('complementary', { name: 'Project navigation' });
    const links = within(sidebar).getAllByRole('link');
    expect(links.map((a) => a.getAttribute('href'))).toEqual([
      '/', // the Marreq logo opens the start screen (issue #387)
      '/space-project/dashboard',
      '/space-project/requirements',
      '/space-project/verifications',
      '/space-project/traceability',
      '/space-project/baselines',
      '/space-project/reports',
      '/space-project/settings',
      '/space-project/help',
    ]);
    for (const gone of [/matrix/i, /^import$/i, /catalog/i, /system logs/i, /backup/i, /groups/i, /new project/i]) {
      expect(within(sidebar).queryByRole('link', { name: gone })).not.toBeInTheDocument();
    }
    expect(within(sidebar).getByRole('link', { name: /project settings/i })).toBeInTheDocument();
    expect(screen.queryByTitle('Settings')).not.toBeInTheDocument();
  });

  it('marks Project settings active on any settings tab', () => {
    renderAt('/space-project/settings/catalog/categories');
    expect(screen.getByRole('link', { name: /project settings/i })).toHaveAttribute('aria-current', 'page');
  });

  it('remembers the collapsed sidebar', async () => {
    const user = userEvent.setup();
    const { unmount } = renderAt();
    await user.click(screen.getByRole('button', { name: /collapse/i }));
    expect(localStorage.getItem('marreq.sidebar.wide')).toBe('0');
    expect(screen.getByRole('link', { name: /requirements/i })).toHaveAttribute('title', 'Requirements');
    unmount();
    renderAt();
    expect(screen.getByRole('button', { name: '»' })).toBeInTheDocument();
  });

  it('opens and closes the mobile drawer', async () => {
    const user = userEvent.setup();
    renderAt();
    const sidebar = screen.getByRole('complementary', { name: 'Project navigation' });
    expect(sidebar).toHaveAttribute('data-drawer-open', 'false');
    await user.click(screen.getByRole('button', { name: 'Open navigation' }));
    expect(sidebar).toHaveAttribute('data-drawer-open', 'true');
    await user.click(screen.getByTestId('sidebar-backdrop'));
    expect(sidebar).toHaveAttribute('data-drawer-open', 'false');
    await user.click(screen.getByRole('button', { name: 'Open navigation' }));
    await act(async () => {
      await user.click(within(sidebar).getByRole('link', { name: /baselines/i }));
    });
    expect(screen.getByTestId('where')).toHaveTextContent('/space-project/baselines');
    expect(sidebar).toHaveAttribute('data-drawer-open', 'false');
  });

  it('switches projects and reaches groups and new project from the project menu', async () => {
    const user = userEvent.setup();
    renderAt('/space-project/requirements');
    await user.click(screen.getByRole('button', { name: 'Switch project' }));
    const menu = screen.getByRole('menu', { name: 'Projects' });
    expect(within(menu).getByRole('menuitem', { name: /groups/i })).toHaveAttribute('href', '/groups');
    expect(within(menu).getByRole('menuitem', { name: /new project/i })).toHaveAttribute('href', '/projects/new');
    await user.click(within(menu).getByRole('menuitemradio', { name: /rover/i }));
    expect(screen.getByTestId('where')).toHaveTextContent('/rover/requirements');
  });

  it('offers Administration in the avatar menu to instance admins only', async () => {
    const user = userEvent.setup();
    const { unmount } = renderAt();
    await user.click(screen.getByRole('button', { name: 'User menu' }));
    expect(screen.getByRole('menuitem', { name: 'Administration' })).toHaveAttribute('href', '/admin');
    unmount();

    dashboard.value.user.is_admin = false;
    renderAt();
    await user.click(screen.getByRole('button', { name: 'User menu' }));
    expect(screen.queryByRole('menuitem', { name: 'Administration' })).not.toBeInTheDocument();
  });

  it('points Create → Import at Project settings', async () => {
    const user = userEvent.setup();
    renderAt();
    await user.click(screen.getByRole('button', { name: 'Open create menu' }));
    expect(screen.getByRole('menuitem', { name: /import/i })).toHaveAttribute(
      'href',
      '/space-project/settings/import',
    );
  });

  it('lists archived projects apart, collapsed until asked', async () => {
    const user = userEvent.setup();
    renderAt('/space-project/dashboard');
    await user.click(screen.getByRole('button', { name: 'Switch project' }));
    const menu = screen.getByRole('menu', { name: 'Projects' });
    expect(within(menu).queryByRole('menuitemradio', { name: /old probe/i })).not.toBeInTheDocument();
    const toggle = within(menu).getByRole('button', { name: /archived \(1\)/i });
    expect(toggle).toHaveAttribute('aria-expanded', 'false');
    await user.click(toggle);
    await user.click(within(menu).getByRole('menuitemradio', { name: /old probe/i }));
    expect(screen.getByTestId('where')).toHaveTextContent('/old-probe/dashboard');
  });

  it('marks an archived project read-only and lets an instance admin unarchive it', async () => {
    const user = userEvent.setup();
    vi.mocked(apiClient.setProjectArchived).mockResolvedValue({} as never);
    renderAt('/old-probe/requirements');
    expect(screen.getByText('This project is archived.')).toBeInTheDocument();
    expect(screen.queryByRole('link', { name: /create requirement/i })).not.toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Unarchive' }));
    expect(apiClient.setProjectArchived).toHaveBeenCalledWith(7, false, 'csrf');
    expect(refresh).toHaveBeenCalled();
  });

  it('offers Unarchive only to the owner or an instance admin', () => {
    dashboard.value.user.is_admin = false;
    renderAt('/old-probe/dashboard');
    expect(screen.getByText(/read-only for everyone\.$/)).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Unarchive' })).not.toBeInTheDocument();
  });

  it('shows no banner on an active project', () => {
    renderAt('/space-project/dashboard');
    expect(screen.queryByText('This project is archived.')).not.toBeInTheDocument();
    expect(screen.getByRole('link', { name: /create requirement/i })).toBeInTheDocument();
  });

  it('records the page for the start screen\'s Recent list', () => {
    renderAt('/rover/traceability?view=matrix');
    expect(readRecentPages()[0]).toMatchObject({
      projectId: 6,
      path: '/rover/traceability?view=matrix',
      section: 'Traceability',
    });
    expect(screen.getByRole('link', { name: 'Marreq start screen' })).toHaveAttribute('href', '/');
  });
});
