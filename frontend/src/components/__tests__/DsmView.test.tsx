import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes, useLocation } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as apiClient from '@/api/client';
import type { Dsm } from '@/api/types';
import DsmView from '../dsm/DsmView';
import { DSM_CELL } from '../dsm/DsmGrid';

vi.mock('@/api/client');

const dsm: Dsm = {
  order: 'hierarchy',
  link_types: ['DERIVES_FROM', 'REFINES', 'DEPENDS_ON', 'SATISFIES'],
  requirements: [
    { id: 1, index: 0, reference_code: 'REQ-1', title: 'Power budget', approval_state: 'approved', category_id: 1, category: 'Power', parent_id: null, depth: 0 },
    { id: 2, index: 1, reference_code: 'REQ-2', title: 'Battery', approval_state: 'approved', category_id: 1, category: 'Power', parent_id: 1, depth: 1 },
    { id: 3, index: 2, reference_code: 'REQ-3', title: 'Heater', approval_state: 'draft', category_id: 1, category: 'Power', parent_id: 1, depth: 1 },
  ],
  cells: [
    { row: 1, col: 0, link_types: ['DERIVES_FROM'], link_ids: [10], upstream_changed: true, in_loop: false },
    { row: 1, col: 2, link_types: ['DEPENDS_ON'], link_ids: [11], upstream_changed: false, in_loop: true },
    { row: 2, col: 1, link_types: ['DEPENDS_ON', 'REFINES'], link_ids: [12, 13], upstream_changed: false, in_loop: true },
  ],
  groups: [{ label: 'Power', start: 0, end: 2 }],
  loops: [{ requirement_ids: [2, 3], path: [2, 3] }],
  stats: { requirements: 3, links: 4, cells: 3, loops: 1, upstream_changed: 1, external_links: 0 },
};

function LocationProbe() {
  const loc = useLocation();
  return <output data-testid="location">{loc.search}</output>;
}

function renderView(search = '?view=dsm') {
  return render(
    <MemoryRouter initialEntries={[`/space/traceability${search}`]}>
      <Routes>
        <Route
          path="/:projectSlug/traceability"
          element={
            <>
              <DsmView projectId={5} basePath="/space" />
              <LocationProbe />
            </>
          }
        />
        <Route path="/:projectSlug/requirements/:id" element={<p>requirement page</p>} />
      </Routes>
    </MemoryRouter>,
  );
}

beforeEach(() => {
  vi.mocked(apiClient.getDsm).mockReset().mockResolvedValue(dsm);
  vi.mocked(apiClient.listCategories).mockResolvedValue([
    { id: 1, title: 'Power', description: '', tag: '', project_id: 5 },
    { id: 9, title: 'Other project', description: '', tag: '', project_id: 6 },
  ]);
  vi.mocked(apiClient.listRequirements).mockResolvedValue([]);
  vi.mocked(apiClient.downloadDsmXlsx).mockReset().mockResolvedValue(undefined);
});

describe('DsmView', () => {
  it('draws one mark per cell at its row and column, with stats and loops', async () => {
    renderView();
    expect(await screen.findByTestId('dsm-cell-1-0')).toHaveTextContent('D');
    expect(screen.getByTestId('dsm-cell-2-1')).toHaveTextContent('PR');
    expect(screen.getByTestId('dsm-stats')).toHaveTextContent('3 req · 4 links · 1 loop · 1 upstream changed');
    expect(screen.getByText(/LOOP 1 · 2 requirements/)).toBeInTheDocument();
    // Only this project's categories are offered as scope.
    await waitFor(() => expect(screen.getByRole('option', { name: 'Power' })).toBeInTheDocument());
    expect(screen.queryByRole('option', { name: 'Other project' })).not.toBeInTheDocument();
  });

  it('updates the URL and request when a link type or the order changes', async () => {
    const user = userEvent.setup();
    renderView();
    await screen.findByTestId('dsm-cell-1-0');

    await user.click(screen.getByRole('button', { name: /Relates to/ }));
    await waitFor(() =>
      expect(apiClient.getDsm).toHaveBeenLastCalledWith(5, expect.objectContaining({
        linkTypes: ['DERIVES_FROM', 'REFINES', 'DEPENDS_ON', 'SATISFIES', 'RELATES_TO'],
      })),
    );
    expect(screen.getByTestId('location').textContent).toContain('dsm_types=');

    await user.click(screen.getByRole('button', { name: 'Partition' }));
    await waitFor(() =>
      expect(apiClient.getDsm).toHaveBeenLastCalledWith(5, expect.objectContaining({ order: 'partition' })),
    );
    expect(screen.getByTestId('location').textContent).toContain('dsm_order=partition');
    expect(screen.getByTestId('location').textContent).toContain('view=dsm');
  });

  it('sizes the grid to its content so the sticky row headers stay visible when scrolled', async () => {
    renderView();
    const grid = await screen.findByTestId('dsm-grid');
    // Sticky elements cannot leave their containing block; a viewport-wide grid
    // would let the row headers scroll away horizontally (issue #332).
    expect(grid.firstElementChild).toHaveClass('w-max');
  });

  it('shows a hover card for a marked cell, including the upstream change', async () => {
    renderView();
    const body = await screen.findByTestId('dsm-body');
    fireEvent.mouseMove(body, { clientX: 0 * DSM_CELL + 5, clientY: 1 * DSM_CELL + 5 });
    const card = await screen.findByRole('tooltip');
    expect(card).toHaveTextContent('REQ-2 → REQ-1');
    expect(card).toHaveTextContent('Derives from');
    expect(card).toHaveTextContent('Upstream changed');
    // Both requirements are named, not just the target.
    expect(card).toHaveTextContent('REQ-2: Battery');
    expect(card).toHaveTextContent('REQ-1: Power budget');
  });

  it('shows the requirement title when hovering a row header', async () => {
    renderView();
    fireEvent.mouseEnter(await screen.findByTestId('dsm-row-2'), { clientX: 40, clientY: 300 });
    const card = await screen.findByRole('tooltip');
    expect(card).toHaveTextContent('REQ-3');
    expect(card).toHaveTextContent('Heater');
    expect(card).toHaveTextContent('Power · draft · parent REQ-1');
    expect(card).toHaveTextContent('Depends on 1 · Used by 1');
    fireEvent.mouseLeave(screen.getByTestId('dsm-row-2'));
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument();
  });

  it('shows the requirement title when hovering a column header', async () => {
    renderView();
    const header = await screen.findByTestId('dsm-col-0');
    expect(header).not.toHaveAttribute('title');
    fireEvent.mouseEnter(header, { clientX: 400, clientY: 100 });
    const card = await screen.findByRole('tooltip');
    expect(card).toHaveTextContent('REQ-1');
    expect(card).toHaveTextContent('Power budget');
    expect(card).toHaveTextContent('Depends on 0 · Used by 1');
    expect(header).toHaveClass('bg-stitch-accent/10');
  });

  it('opens the source requirement when a mark is clicked', async () => {
    renderView();
    const body = await screen.findByTestId('dsm-body');
    fireEvent.click(body, { clientX: 2 * DSM_CELL + 5, clientY: 1 * DSM_CELL + 5 });
    expect(await screen.findByText('requirement page')).toBeInTheDocument();
  });

  it('exports with the current filters', async () => {
    const user = userEvent.setup();
    renderView('?view=dsm&dsm_order=partition');
    await screen.findByTestId('dsm-cell-1-0');
    await user.click(screen.getByRole('button', { name: 'Export Excel' }));
    expect(apiClient.downloadDsmXlsx).toHaveBeenCalledWith(5, expect.objectContaining({ order: 'partition' }));
  });

  it('shows the API error', async () => {
    vi.mocked(apiClient.getDsm).mockRejectedValue(new Error('unknown link type: BOGUS'));
    renderView('?view=dsm&dsm_types=BOGUS');
    expect(await screen.findByRole('alert')).toHaveTextContent('unknown link type: BOGUS');
  });
});
