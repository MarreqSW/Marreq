import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes, useLocation, useOutletContext } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import TraceabilityPage from '../TraceabilityPage';
import { MatrixRedirect } from '../LegacyRedirects';

vi.mock('@/components/TraceabilityGraph', () => ({ default: () => <p>coverage graph</p> }));
vi.mock('@/components/HierarchyGraph', () => ({ default: () => <p>hierarchy graph</p> }));
vi.mock('@/components/dsm/DsmView', () => ({ default: () => <p>dsm view</p> }));
vi.mock('@/components/matrix/MatrixView', () => ({ default: () => <p>matrix view</p> }));
vi.mock('@/components/RequirementsViewSwitcher', () => ({ default: () => null }));
vi.mock('@/context/DashboardContext', () => ({
  useDashboard: () => ({ dashboard: { projects: [{ id: 5, name: 'Space Project' }] } }),
}));
vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual<typeof import('react-router-dom')>('react-router-dom');
  return { ...actual, useOutletContext: vi.fn() };
});

function LocationProbe() {
  return <output data-testid="location">{useLocation().search}</output>;
}

function renderPage(search = '') {
  return render(
    <MemoryRouter initialEntries={[`/space/traceability${search}`]}>
      <Routes>
        <Route
          path="/:projectSlug/traceability"
          element={
            <>
              <TraceabilityPage />
              <LocationProbe />
            </>
          }
        />
      </Routes>
    </MemoryRouter>,
  );
}

beforeEach(() => {
  vi.mocked(useOutletContext).mockReturnValue({
    projectId: 5,
    basePath: '/space',
    globalSearch: '',
    setGlobalSearch: vi.fn(),
  });
});

describe('TraceabilityPage views', () => {
  it('opens the DSM from ?view=dsm', () => {
    renderPage('?view=dsm');
    expect(screen.getByText('dsm view')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /DSM/ })).toHaveAttribute('aria-pressed', 'true');
  });

  it('switches to the DSM tab and back to coverage (default, no param)', async () => {
    const user = userEvent.setup();
    renderPage();
    expect(screen.getByText('coverage graph')).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: /DSM/ }));
    expect(screen.getByText('dsm view')).toBeInTheDocument();
    expect(screen.getByTestId('location')).toHaveTextContent('?view=dsm');
    await user.click(screen.getByRole('button', { name: /Coverage/ }));
    expect(screen.getByText('coverage graph')).toBeInTheDocument();
    expect(screen.getByTestId('location')).toBeEmptyDOMElement();
  });

  it('shows the matrix as a tab and keeps its filters when switching', async () => {
    const user = userEvent.setup();
    renderPage('?view=matrix&mx_suspect=1');
    expect(screen.getByText('matrix view')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Matrix/ })).toHaveAttribute('aria-pressed', 'true');
    await user.click(screen.getByRole('button', { name: /DSM/ }));
    expect(screen.getByTestId('location')).toHaveTextContent('?view=dsm&mx_suspect=1');
    await user.click(screen.getByRole('button', { name: /Matrix/ }));
    expect(screen.getByText('matrix view')).toBeInTheDocument();
  });
});

describe('MatrixRedirect', () => {
  it('sends the old /matrix URL to the Traceability matrix tab with its filters', () => {
    render(
      <MemoryRouter initialEntries={['/space/matrix?mx_sort=V-1&mx_dir=desc']}>
        <Routes>
          <Route path="/:projectSlug/matrix" element={<MatrixRedirect />} />
          <Route path="/:projectSlug/traceability" element={<LocationProbe />} />
        </Routes>
      </MemoryRouter>,
    );
    expect(screen.getByTestId('location')).toHaveTextContent('?mx_sort=V-1&mx_dir=desc&view=matrix');
  });
});
