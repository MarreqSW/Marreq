import { render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter, Route, Routes, useLocation } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as apiClient from '@/api/client';
import { MovedRedirect } from '@/pages/LegacyRedirects';
import AdminLayout from '../AdminLayout';

vi.mock('@/api/client');

function Where() {
  const { pathname, search } = useLocation();
  return <output data-testid="where">{pathname + search}</output>;
}

function renderAt(entry: string | { pathname: string; state?: unknown }) {
  return render(
    <MemoryRouter initialEntries={[entry]}>
      <Routes>
        <Route path="/admin" element={<AdminLayout />}>
          <Route index element={<p>users page</p>} />
          <Route path="logs" element={<p>logs page</p>} />
          <Route path="logs/analytics" element={<p>analytics page</p>} />
          <Route path="backup" element={<p>backup page</p>} />
        </Route>
        <Route path="/:projectSlug/admin/*" element={<MovedRedirect to="/admin" />} />
      </Routes>
      <Where />
    </MemoryRouter>,
  );
}

beforeEach(() => {
  vi.resetAllMocks();
  sessionStorage.clear();
  vi.mocked(apiClient.getDeploymentInfo).mockResolvedValue({ mode: 'server', allows_database_backup: true } as Awaited<
    ReturnType<typeof apiClient.getDeploymentInfo>
  >);
});

describe('AdminLayout', () => {
  it('shows the administration tabs and the active page', async () => {
    renderAt('/admin/logs');
    expect(screen.getByText('logs page')).toBeInTheDocument();
    const nav = screen.getByRole('navigation', { name: 'Administration sections' });
    await waitFor(() => expect(nav.querySelectorAll('a')).toHaveLength(4));
    expect(screen.getByRole('link', { name: /system logs/i })).toHaveAttribute('aria-current', 'page');
    expect(screen.getByRole('link', { name: 'Users' })).not.toHaveAttribute('aria-current');
  });

  it('hides Backup when the deployment does not allow database backups', async () => {
    vi.mocked(apiClient.getDeploymentInfo).mockResolvedValue({ mode: 'cloud', allows_database_backup: false } as Awaited<
      ReturnType<typeof apiClient.getDeploymentInfo>
    >);
    renderAt('/admin');
    await waitFor(() => expect(screen.queryByRole('link', { name: /backup/i })).not.toBeInTheDocument());
    expect(screen.getByRole('link', { name: /log analytics/i })).toBeInTheDocument();
  });

  it('returns to the project it was opened from', () => {
    renderAt({ pathname: '/admin', state: { from: '/space-project' } });
    expect(screen.getByRole('link', { name: /back to project/i })).toHaveAttribute('href', '/space-project');
  });

  it('redirects the old project-scoped admin URLs, keeping the query', async () => {
    renderAt('/space-project/admin/logs?action_type=UPDATE');
    await waitFor(() => expect(screen.getByTestId('where')).toHaveTextContent('/admin/logs?action_type=UPDATE'));
    expect(screen.getByText('logs page')).toBeInTheDocument();
  });
});
