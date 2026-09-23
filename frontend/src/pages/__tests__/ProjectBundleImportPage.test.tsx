import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import ProjectBundleImportPage from '../ProjectBundleImportPage';

const mocks = vi.hoisted(() => ({
  importProjectBundle: vi.fn(),
  listCreatableGroups: vi.fn(),
  navigate: vi.fn(),
  refresh: vi.fn(),
}));

vi.mock('@/api/client', () => ({
  importProjectBundle: mocks.importProjectBundle,
  listCreatableGroups: mocks.listCreatableGroups,
}));

vi.mock('@/context/DashboardContext', () => ({
  useDashboard: () => ({
    csrfToken: 'csrf-token',
    dashboard: {
      user: { id: 1, username: 'alice', name: 'Alice', is_admin: false },
      projects: [],
    },
    refresh: mocks.refresh,
  }),
}));

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual<typeof import('react-router-dom')>('react-router-dom');
  return { ...actual, useNavigate: () => mocks.navigate };
});

describe('ProjectBundleImportPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.listCreatableGroups.mockResolvedValue([]);
    mocks.refresh.mockResolvedValue(undefined);
    mocks.importProjectBundle.mockResolvedValue({
      project_id: 9,
      slug: 'imported',
      project_base_path: '/imported',
      imported_counts: { requirements: 1 },
      warnings: [],
      errors: [],
    });
  });

  it('imports a JSON bundle and opens the new project', async () => {
    const user = userEvent.setup();
    render(
      <MemoryRouter>
        <ProjectBundleImportPage />
      </MemoryRouter>,
    );

    const file = new File(['{"format":"marreq.project-bundle.v1"}'], 'bundle.json', {
      type: 'application/json',
    });
    await user.upload(await screen.findByLabelText(/bundle file/i), file);
    await user.click(screen.getByRole('button', { name: /import bundle/i }));

    await waitFor(() =>
      expect(mocks.importProjectBundle).toHaveBeenCalledWith(file, 'csrf-token', null),
    );
    expect(mocks.navigate).toHaveBeenCalledWith('/imported/dashboard', { replace: true });
  });
});
