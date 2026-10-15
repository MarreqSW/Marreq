import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes, useLocation } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { ProjectOutletContext } from '@/types/projectOutlet';
import UseCasesPage from '../UseCasesPage';

const mocks = vi.hoisted(() => ({
  listUseCases: vi.fn(),
  getNextUseCaseCode: vi.fn(),
  createUseCase: vi.fn(),
}));
vi.mock('@/api/client', () => mocks);
vi.mock('@/context/DashboardContext', () => ({
  useDashboard: () => ({
    csrfToken: 'csrf',
    dashboard: { projects: [{ id: 5, name: 'Space Project' }] },
  }),
}));
vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual<typeof import('react-router-dom')>('react-router-dom');
  return { ...actual, useOutletContext: vi.fn() };
});
import { useOutletContext } from 'react-router-dom';

function Where() {
  const l = useLocation();
  return <output data-testid="where">{l.pathname + l.search}</output>;
}

function renderPage(path = '/space-project/use-cases') {
  vi.mocked(useOutletContext).mockReturnValue({
    projectId: 5,
    basePath: '/space-project',
    globalSearch: '',
    setGlobalSearch: vi.fn(),
  } satisfies ProjectOutletContext);
  return render(
    <MemoryRouter initialEntries={[path]}>
      <Routes>
        <Route path="/space-project/use-cases" element={<UseCasesPage />} />
        <Route path="*" element={<Where />} />
      </Routes>
    </MemoryRouter>,
  );
}

const row = {
  id: 7,
  reference_code: 'UC-004',
  title: 'Downlink telemetry pass',
  primary_actor: 'Ground operator',
  step_count: 9,
  alternative_count: 3,
  exception_count: 2,
  approval_state: 'reviewed',
  current_version_number: 2,
  has_draft: true,
  updated_at: '2026-10-15T08:00:00',
};

describe('UseCasesPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.listUseCases.mockResolvedValue([row]);
    mocks.getNextUseCaseCode.mockResolvedValue('UC-008');
  });

  it('lists use cases with their counts and state', async () => {
    renderPage();
    const link = await screen.findByRole('link', { name: 'UC-004' });
    expect(link).toHaveAttribute('href', '/space-project/use-cases/7');
    expect(screen.getByText('Ground operator')).toBeInTheDocument();
    expect(screen.getByText('3 · 2')).toBeInTheDocument();
    expect(screen.getByText('Reviewed · v2')).toBeInTheDocument();
    expect(screen.getByText('Editing')).toBeInTheDocument();
  });

  it('filters and sorts on the server', async () => {
    const user = userEvent.setup();
    renderPage();
    await screen.findByText('UC-004');
    await user.selectOptions(screen.getByLabelText('Approval state'), 'approved');
    await user.click(screen.getByRole('button', { name: /title/i }));
    await waitFor(() =>
      expect(mocks.listUseCases).toHaveBeenLastCalledWith(5, {
        q: undefined,
        approval_state: 'approved',
        sort_column: 'title',
        sort_dir: 'asc',
      }),
    );
  });

  it('creates a use case with the suggested code and opens the editor', async () => {
    const user = userEvent.setup();
    mocks.createUseCase.mockResolvedValue({ id: 9 });
    renderPage('/space-project/use-cases?new=1');
    expect(await screen.findByRole('dialog')).toBeInTheDocument();
    await waitFor(() => expect(screen.getByLabelText('Code')).toHaveValue('UC-008'));
    await user.type(screen.getByLabelText('Title'), 'Safe mode entry');
    await user.type(screen.getByLabelText('Primary actor'), 'On-board software');
    await user.click(screen.getByRole('button', { name: 'Create and edit' }));
    expect(mocks.createUseCase).toHaveBeenCalledWith(
      5,
      { reference_code: 'UC-008', title: 'Safe mode entry', primary_actor: 'On-board software' },
      'csrf',
    );
    expect(await screen.findByTestId('where')).toHaveTextContent('/space-project/use-cases/9?edit=1');
  });
});
