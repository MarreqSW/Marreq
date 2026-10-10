import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes, useLocation } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as apiClient from '@/api/client';
import type { DashboardProject } from '@/api/types';
import StartSearch from '../start/StartSearch';

vi.mock('@/api/client');

const projects: DashboardProject[] = [
  { id: 5, name: 'Space Project', slug: 'space-project', project_base_path: '/space-project', group_id: null, group_name: 'Satellite Team', group_slug: null },
  { id: 6, name: 'Thermal Subsystem', slug: 'thermal', project_base_path: '/thermal', group_id: null, group_name: null, group_slug: null },
];

function Where() {
  return <p data-testid="where">{useLocation().pathname}</p>;
}

function renderSearch(isAdmin = false) {
  return render(
    <MemoryRouter initialEntries={['/']}>
      <Routes>
        <Route path="/" element={<StartSearch projects={projects} isAdmin={isAdmin} />} />
        <Route path="*" element={<Where />} />
      </Routes>
    </MemoryRouter>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(apiClient.searchRequirements).mockResolvedValue([]);
});

describe('StartSearch', () => {
  it('finds projects and requirements, and opens one with the keyboard', async () => {
    vi.mocked(apiClient.searchRequirements).mockResolvedValue([
      { project_id: 6, requirement_id: 40, reference_code: 'THM-004', title: 'Radiator area' },
      { project_id: 99, requirement_id: 1, reference_code: 'X-1', title: 'Not listed' },
      { project_id: 6, requirement_id: 41, reference_code: 'THM-005', title: 'THM-005' },
    ]);
    const user = userEvent.setup();
    renderSearch();
    const box = screen.getByRole('combobox', { name: /search projects, requirements and actions/i });
    expect(box).toHaveFocus();
    await user.type(box, 'therm');

    expect(screen.getByRole('option', { name: /thermal subsystem/i })).toBeInTheDocument();
    expect(await screen.findByRole('option', { name: /THM-004 · Radiator area/ })).toBeInTheDocument();
    expect(screen.queryByRole('option', { name: /X-1/ })).not.toBeInTheDocument();
    expect(screen.getByRole('option', { name: /^THM-005 Thermal Subsystem$/ })).toBeInTheDocument();
    expect(apiClient.searchRequirements).toHaveBeenLastCalledWith('therm');

    await user.keyboard('{ArrowDown}{Enter}');
    expect(screen.getByTestId('where')).toHaveTextContent('/thermal/requirements/40');
  });

  it('matches actions, keeps Administration for admins, and waits for two characters', async () => {
    const user = userEvent.setup();
    const { unmount } = renderSearch();
    const box = screen.getByRole('combobox');
    await user.type(box, 'a');
    expect(screen.getByRole('option', { name: /space project/i })).toBeInTheDocument();
    expect(apiClient.searchRequirements).not.toHaveBeenCalled();
    await user.clear(box);
    await user.type(box, 'admin');
    expect(screen.queryByRole('option', { name: 'Administration' })).not.toBeInTheDocument();
    expect(await screen.findByText('No matches.')).toBeInTheDocument();
    unmount();

    renderSearch(true);
    await user.type(screen.getByRole('combobox'), 'admin');
    await user.click(screen.getByRole('option', { name: 'Administration' }));
    expect(screen.getByTestId('where')).toHaveTextContent('/admin');
  });

  it('focuses with Ctrl+K and clears with Escape', async () => {
    const user = userEvent.setup();
    renderSearch();
    const box = screen.getByRole('combobox');
    box.blur();
    await user.keyboard('{Control>}k{/Control}');
    expect(box).toHaveFocus();
    await user.type(box, 'space');
    expect(box).toHaveAttribute('aria-expanded', 'true');
    await user.keyboard('{Escape}');
    expect(box).toHaveValue('');
    expect(box).toHaveAttribute('aria-expanded', 'false');
  });
});
