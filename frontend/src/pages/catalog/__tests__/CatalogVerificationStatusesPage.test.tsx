import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Outlet, Route, Routes } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as apiClient from '@/api/client';
import type { EffectivePermissions, VerificationStatus } from '@/api/types';
import CatalogVerificationStatusesPage from '../CatalogVerificationStatusesPage';

vi.mock('@/api/client');
vi.mock('@/context/DashboardContext', () => ({
  useDashboard: () => ({ csrfToken: 'csrf' }),
}));

const status = (patch: Partial<VerificationStatus>): VerificationStatus => ({
  id: 1,
  title: 'Passed',
  description: '',
  tag: 'P',
  project_id: 5,
  is_system: true,
  tag_color: null,
  outcome: 'passed',
  ...patch,
});

function renderPage() {
  return render(
    <MemoryRouter initialEntries={['/p/catalog']}>
      <Routes>
        <Route
          path="/p"
          element={<Outlet context={{ projectId: 5, basePath: '/p', globalSearch: '', setGlobalSearch: vi.fn() }} />}
        >
          <Route path="catalog" element={<CatalogVerificationStatusesPage />} />
        </Route>
      </Routes>
    </MemoryRouter>,
  );
}

describe('CatalogVerificationStatusesPage outcome (issue #353)', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(apiClient.listVerificationStatuses).mockResolvedValue([
      status({}),
      status({ id: 2, title: 'Waived', tag: 'W', is_system: false, outcome: 'not_run' }),
      status({ id: 3, title: 'Other project', project_id: 6 }),
    ]);
    vi.mocked(apiClient.getMyPermissions).mockResolvedValue({ edit_requirements: true } as EffectivePermissions);
    vi.mocked(apiClient.updateVerificationStatus).mockResolvedValue(undefined);
    vi.mocked(apiClient.createVerificationStatus).mockResolvedValue({ id: 4 });
  });

  it('shows each outcome, locks system statuses and saves a changed outcome', async () => {
    renderPage();
    const waived = await screen.findByLabelText('Outcome of Waived');
    expect(waived).toHaveValue('not_run');
    expect(screen.getByLabelText('Outcome of Passed')).toBeDisabled();
    expect(screen.queryByLabelText('Outcome of Other project')).not.toBeInTheDocument();

    await userEvent.selectOptions(waived, 'passed');
    const saveButtons = screen.getAllByRole('button', { name: 'Save' });
    // Rows are sorted by title: Passed, Waived.
    await userEvent.click(saveButtons[1]);
    expect(apiClient.updateVerificationStatus).toHaveBeenCalledWith(
      2,
      expect.objectContaining({ title: 'Waived', outcome: 'passed' }),
      'csrf',
    );
  });

  it('creates a status with an explicit outcome, or lets the server infer it', async () => {
    renderPage();
    await screen.findByLabelText('Outcome of Waived');
    await userEvent.type(screen.getByPlaceholderText('Title (e.g. Passed)'), 'Rejected');
    await userEvent.selectOptions(screen.getByLabelText('Close-out outcome'), 'failed');
    await userEvent.click(screen.getByRole('button', { name: 'Add status' }));
    expect(apiClient.createVerificationStatus).toHaveBeenLastCalledWith(
      expect.objectContaining({ title: 'Rejected', outcome: 'failed' }),
      'csrf',
    );

    await userEvent.type(await screen.findByPlaceholderText('Title (e.g. Passed)'), 'Blocked');
    await userEvent.click(screen.getByRole('button', { name: 'Add status' }));
    const body = vi.mocked(apiClient.createVerificationStatus).mock.calls.at(-1)?.[0];
    expect(body).toMatchObject({ title: 'Blocked' });
    expect(body).not.toHaveProperty('outcome');
  });
});
