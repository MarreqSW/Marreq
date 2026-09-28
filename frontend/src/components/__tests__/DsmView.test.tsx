import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes, useLocation } from 'react-router-dom';
import { beforeEach, describe, expect, it, onTestFinished, vi } from 'vitest';
import * as apiClient from '@/api/client';
import type { Dsm } from '@/api/types';
import DsmView from '../dsm/DsmView';
import { DSM_CELL, focusScrollPosition } from '../dsm/DsmGrid';

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

  it('focuses the matrix on a loop when it is clicked, and clears it on a second click', async () => {
    const scrollTo = vi.fn();
    const originalScrollTo = HTMLElement.prototype.scrollTo;
    HTMLElement.prototype.scrollTo = scrollTo;
    onTestFinished(() => {
      HTMLElement.prototype.scrollTo = originalScrollTo;
    });
    const user = userEvent.setup();
    renderView();
    const loop = await screen.findByTestId('dsm-loop-0');
    await user.click(loop);

    expect(loop).toHaveAttribute('aria-pressed', 'true');
    // Loop members REQ-2 and REQ-3 sit at indices 1..2: the frame spans them (3 px padding).
    const frame = screen.getByTestId('dsm-focus');
    expect(frame.style.left).toBe(`${1 * DSM_CELL - 3}px`);
    expect(frame.style.top).toBe(`${1 * DSM_CELL - 3}px`);
    expect(frame.style.width).toBe(`${2 * DSM_CELL + 6}px`);
    expect(scrollTo).toHaveBeenCalledWith(expect.objectContaining({ behavior: 'smooth' }));

    await user.click(loop);
    expect(loop).toHaveAttribute('aria-pressed', 'false');
    expect(screen.queryByTestId('dsm-focus')).not.toBeInTheDocument();
  });

  it('focuses the matrix on an upstream-changed finding, also from the keyboard', async () => {
    const user = userEvent.setup();
    renderView();
    const finding = await screen.findByTestId('dsm-finding-1-0');
    finding.focus();
    await user.keyboard('{Enter}');
    expect(finding).toHaveAttribute('aria-pressed', 'true');
    const frame = screen.getByTestId('dsm-focus');
    expect(frame.style.left).toBe(`${0 * DSM_CELL - 3}px`);
    expect(frame.style.top).toBe(`${1 * DSM_CELL - 3}px`);
    expect(frame.style.width).toBe(`${DSM_CELL + 6}px`);
  });

  it('switches to Partition order when a loop is spread out, then focuses it', async () => {
    // Hierarchy order: loop members REQ-1 and REQ-3 are two rows apart with REQ-2 between them.
    const spread: Dsm = {
      ...dsm,
      cells: [
        { row: 0, col: 2, link_types: ['DEPENDS_ON'], link_ids: [20], upstream_changed: false, in_loop: true },
        { row: 2, col: 0, link_types: ['DEPENDS_ON'], link_ids: [21], upstream_changed: false, in_loop: true },
      ],
      loops: [{ requirement_ids: [1, 3], path: [1, 3] }],
      stats: { ...dsm.stats, cells: 2, links: 2, upstream_changed: 0 },
    };
    // Partition order: the loop members are adjacent (indices 1 and 2).
    const partitioned: Dsm = {
      ...spread,
      order: 'partition',
      groups: [],
      requirements: [
        { ...dsm.requirements[1]!, index: 0 },
        { ...dsm.requirements[0]!, index: 1 },
        { ...dsm.requirements[2]!, index: 2 },
      ],
      cells: [
        { row: 1, col: 2, link_types: ['DEPENDS_ON'], link_ids: [20], upstream_changed: false, in_loop: true },
        { row: 2, col: 1, link_types: ['DEPENDS_ON'], link_ids: [21], upstream_changed: false, in_loop: true },
      ],
    };
    vi.mocked(apiClient.getDsm).mockImplementation(async (_pid, p) => (p.order === 'partition' ? partitioned : spread));
    const user = userEvent.setup();
    renderView();
    await user.click(await screen.findByTestId('dsm-loop-0'));

    expect(await screen.findByText(/Switched to Partition order/)).toHaveAttribute('role', 'status');
    await waitFor(() => expect(screen.getByTestId('location').textContent).toContain('dsm_order=partition'));
    // After the reload the selection is kept (by requirement ids) and framed compactly.
    await waitFor(() => expect(screen.getByTestId('dsm-focus').style.top).toBe(`${1 * DSM_CELL - 3}px`));
    expect(screen.getByTestId('dsm-focus').style.height).toBe(`${2 * DSM_CELL + 6}px`);
    expect(screen.getByTestId('dsm-loop-0')).toHaveAttribute('aria-pressed', 'true');
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

describe('focusScrollPosition', () => {
  it('centres the focused area in the space left by the sticky headers', () => {
    // Body viewport: 1000 - 300 = 700 wide, 700 - 112 = 588 high.
    const pos = focusScrollPosition({ rows: [40, 41], cols: [60, 61] }, { width: 1000, height: 700 });
    expect(pos).toEqual({ left: 61 * DSM_CELL - 350, top: 41 * DSM_CELL - 294 });
  });

  it('never scrolls to negative offsets', () => {
    expect(focusScrollPosition({ rows: [0, 0], cols: [1, 1] }, { width: 1000, height: 700 })).toEqual({
      left: 0,
      top: 0,
    });
  });
});
