import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as apiClient from '@/api/client';
import { ApiError } from '@/api/transport';
import type { Project } from '@/api/types';
import DeleteProjectSection from '../DeleteProjectSection';

vi.mock('@/api/client');

const refresh = vi.fn();
let user: { id: number; username: string; is_admin: boolean } = {
  id: 1,
  username: 'alice',
  is_admin: false,
};
vi.mock('@/context/DashboardContext', () => ({
  useDashboard: () => ({ dashboard: { user }, csrfToken: 'csrf', refresh }),
}));

const project: Project = {
  id: 5,
  name: 'Space Project',
  slug: 'space-project',
  description: null,
  owner_id: 1,
  group_id: null,
  status: 'Active',
  creation_date: null,
  update_date: null,
};

function renderSection() {
  return render(
    <MemoryRouter initialEntries={['/space-project/settings/general']}>
      <Routes>
        <Route
          path="/space-project/settings/general"
          element={
            <DeleteProjectSection
              projectId={5}
              basePath="/space-project"
              userLabel={(id) => (id === 1 ? 'Alice' : `#${id}`)}
            />
          }
        />
        <Route path="/" element={<p>home page</p>} />
      </Routes>
    </MemoryRouter>,
  );
}

describe('DeleteProjectSection', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    user = { id: 1, username: 'alice', is_admin: false };
    vi.mocked(apiClient.listProjectsOptional).mockResolvedValue([project]);
    vi.mocked(apiClient.deleteProject).mockResolvedValue(undefined);
  });

  it('shows the danger zone to the project owner', async () => {
    renderSection();
    expect(await screen.findByText('Danger zone')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Delete project…' })).toBeInTheDocument();
  });

  it('shows the danger zone to an instance administrator who does not own the project', async () => {
    user = { id: 9, username: 'root', is_admin: true };
    renderSection();
    expect(await screen.findByText('Danger zone')).toBeInTheDocument();
  });

  it('tells other users who may delete the project', async () => {
    user = { id: 2, username: 'bob', is_admin: false };
    renderSection();
    expect(
      await screen.findByText(
        'Only the project owner (Alice) or an instance administrator can delete this project.',
      ),
    ).toBeInTheDocument();
    expect(screen.queryByText('Danger zone')).not.toBeInTheDocument();
  });

  it('deletes only after the slug is typed, then refreshes and goes home', async () => {
    renderSection();
    await userEvent.click(await screen.findByRole('button', { name: 'Delete project…' }));
    const dialog = screen.getByRole('dialog');
    expect(dialog).toHaveTextContent('This cannot be undone.');
    expect(screen.getByRole('link', { name: 'Reports & exports' })).toHaveAttribute(
      'href',
      '/space-project/reports',
    );
    const submit = screen.getByRole('button', { name: 'Delete project' });
    expect(submit).toBeDisabled();
    await userEvent.type(screen.getByLabelText('Project slug'), 'space-projec');
    expect(submit).toBeDisabled();
    await userEvent.type(screen.getByLabelText('Project slug'), 't');
    expect(submit).toBeEnabled();
    await userEvent.click(submit);

    expect(await screen.findByText('home page')).toBeInTheDocument();
    expect(apiClient.deleteProject).toHaveBeenCalledWith(5, 'space-project', 'csrf');
    expect(refresh).toHaveBeenCalled();
  });

  it('shows a server error and stays on the page', async () => {
    vi.mocked(apiClient.deleteProject).mockRejectedValue(
      new ApiError(403, 'only the project owner or an instance administrator can delete this project'),
    );
    renderSection();
    await userEvent.click(await screen.findByRole('button', { name: 'Delete project…' }));
    await userEvent.type(screen.getByLabelText('Project slug'), 'space-project');
    await userEvent.click(screen.getByRole('button', { name: 'Delete project' }));

    await waitFor(() =>
      expect(screen.getByRole('alert')).toHaveTextContent(
        'only the project owner or an instance administrator can delete this project',
      ),
    );
    expect(screen.queryByText('home page')).not.toBeInTheDocument();
    expect(refresh).not.toHaveBeenCalled();
  });
});
