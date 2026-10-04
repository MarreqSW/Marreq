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

  // Issue #341: a bundle.zip is accepted, and skipped files stay readable.
  it('accepts a zip bundle and shows its warnings before opening the project', async () => {
    mocks.importProjectBundle.mockResolvedValue({
      project_id: 9,
      slug: 'imported',
      project_base_path: '/imported',
      imported_counts: { requirements: 1, attachments: 1 },
      warnings: ['requirement REQ-1: huge.pdf is larger than the 10 MB per-file limit; not attached'],
      errors: [],
    });
    const user = userEvent.setup();
    render(
      <MemoryRouter>
        <ProjectBundleImportPage />
      </MemoryRouter>,
    );

    const input = await screen.findByLabelText(/bundle file/i);
    expect(input).toHaveAttribute('accept', expect.stringContaining('.zip'));
    const file = new File(['PK'], 'project-bundle.zip', { type: 'application/zip' });
    await user.upload(input, file);
    await user.click(screen.getByRole('button', { name: /import bundle/i }));

    expect(await screen.findByText(/imported with 1 warning/)).toBeInTheDocument();
    expect(screen.getByText(/huge\.pdf is larger than/)).toBeInTheDocument();
    expect(mocks.navigate).not.toHaveBeenCalled();
    expect(screen.getByRole('button', { name: /import bundle/i })).toBeDisabled();

    await user.click(screen.getByRole('button', { name: 'Open project' }));
    expect(mocks.navigate).toHaveBeenCalledWith('/imported/dashboard', { replace: true });
  });
});
