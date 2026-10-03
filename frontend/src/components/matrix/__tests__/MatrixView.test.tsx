import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes, useLocation, useOutletContext } from 'react-router-dom';
import { beforeEach, describe, expect, it, onTestFinished, vi } from 'vitest';
import * as apiClient from '@/api/client';
import type { MatrixLink, Requirement, Verification } from '@/api/types';
import { MATRIX_CELL } from '@/components/matrix/MatrixGrid';
import MatrixView from '../MatrixView';

vi.mock('@/api/client');
vi.mock('@/context/DashboardContext', () => ({
  useDashboard: () => ({ csrfToken: 'csrf-test', dashboard: { projects: [{ id: 5, name: 'Space Project' }] } }),
}));
vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual<typeof import('react-router-dom')>('react-router-dom');
  return { ...actual, useOutletContext: vi.fn() };
});

function req(id: number, code: string, title: string, category_id: number, status_id = 1): Requirement {
  return {
    id,
    current_version_id: id * 10,
    title,
    description: '',
    status_id,
    author_id: 1,
    reviewer_id: 1,
    reference_code: code,
    category_id,
    parent_id: null,
    creation_date: '2026-01-01T00:00:00',
    update_date: '2026-01-01T00:00:00',
    deadline_date: null,
    applicability_id: 1,
    justification: null,
    project_id: 5,
    approval_state: 'draft',
    approved_by: null,
    approved_at: null,
  };
}

function ver(id: number, code: string, name: string, status_id: number): Verification {
  return {
    id,
    name,
    reference_code: code,
    description: '',
    source: '',
    status_id,
    parent_id: null,
    project_id: 5,
    verification_method_id: 1,
    author_id: 1,
    reviewer_id: 1,
  };
}

function link(req_id: number, verification_id: number, suspect = false): MatrixLink {
  return {
    req_id,
    verification_id,
    creation_date: '2026-01-01T00:00:00',
    project_id: 5,
    suspect,
    suspect_at: suspect ? '2026-09-28T10:00:00' : null,
    suspect_reason: suspect ? 'Requirement updated' : null,
    cleared_by: null,
    cleared_at: null,
    triggering_version_id: null,
    triggering_user_id: null,
  };
}

// Categories: Power (1) and Comms (2). REQ-3 has no verification, TEST-C has no requirement.
const requirements = [
  req(1, 'REQ-PWR-2', 'Battery', 1),
  req(2, 'REQ-COM-1', 'Downlink', 2, 2),
  req(3, 'REQ-PWR-1', 'Array', 1),
];
const verifications = [
  ver(10, 'TEST-A', 'Battery test', 1),
  ver(11, 'TEST-B', 'Downlink test', 2),
  ver(12, 'TEST-C', 'Orphan test', 1),
];
const links = [link(1, 10), link(2, 11, true)];

function LocationProbe() {
  return <output data-testid="location">{useLocation().search}</output>;
}

function renderPage(search = '') {
  return render(
    <MemoryRouter initialEntries={[`/space/matrix${search}`]}>
      <Routes>
        <Route
          path="/:projectSlug/matrix"
          element={
            <>
              <MatrixView />
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
  vi.mocked(useOutletContext).mockReturnValue({ projectId: 5, basePath: '/space', globalSearch: '', setGlobalSearch: vi.fn() });
  vi.mocked(apiClient.listMatrix).mockReset().mockResolvedValue(links);
  vi.mocked(apiClient.listRequirements).mockResolvedValue(requirements);
  vi.mocked(apiClient.listVerifications).mockResolvedValue(verifications);
  vi.mocked(apiClient.listVerificationStatuses).mockResolvedValue([
    { id: 1, title: 'Passed', description: '', tag: 'pass', project_id: 5, is_system: true, tag_color: null },
    { id: 2, title: 'Failed', description: '', tag: 'fail', project_id: 5, is_system: true, tag_color: null },
  ]);
  vi.mocked(apiClient.listRequirementStatuses).mockResolvedValue([
    { id: 1, title: 'Draft', description: '', tag: 'draft', project_id: 5, is_system: true, tag_color: null },
    { id: 2, title: 'Accepted', description: '', tag: 'ok', project_id: 5, is_system: true, tag_color: null },
  ]);
  vi.mocked(apiClient.listVerificationMethodsByProject).mockResolvedValue([
    { id: 1, title: 'Test', description: '', tag: 'test', project_id: 5 },
  ]);
  vi.mocked(apiClient.listCategories).mockResolvedValue([
    { id: 1, title: 'Power', description: '', tag: '', project_id: 5 },
    { id: 2, title: 'Comms', description: '', tag: '', project_id: 5 },
  ]);
  vi.mocked(apiClient.clearTraceabilitySuspect).mockReset().mockResolvedValue(undefined);
  vi.mocked(apiClient.downloadMatrixXlsx).mockReset().mockResolvedValue(undefined);
  vi.mocked(apiClient.listRequirementVersionsByProject).mockReset().mockResolvedValue([]);
});

const rowCodes = () =>
  screen.getAllByTestId(/^matrix-row-\d+$/).map((el) => within(el).getByRole('link').textContent);

describe('MatrixView (DSM-style grid)', () => {
  it('groups rows by category, draws status symbols and summarises the matrix', async () => {
    renderPage();
    await screen.findByTestId('matrix-body');
    // Comms before Power; natural code order inside the Power block.
    expect(rowCodes()).toEqual(['REQ-COM-1', 'REQ-PWR-1', 'REQ-PWR-2']);
    expect(screen.getByText('Comms')).toBeInTheDocument();
    // REQ-PWR-2 (row 2) × TEST-A (col 0) passed; REQ-COM-1 (row 0) × TEST-B (col 1) failed and suspect.
    expect(screen.getByTestId('matrix-cell-2-0')).toHaveTextContent('✓');
    expect(screen.getByTestId('matrix-cell-0-1')).toHaveTextContent('✗');
    expect(screen.getByTestId('matrix-stats')).toHaveTextContent(
      '3 req × 3 verifications · 2 links · 1 suspect · 2 coverage gaps',
    );
  });

  it('lists coverage gaps and focuses the row or column when clicked', async () => {
    const user = userEvent.setup();
    renderPage();
    await user.click(await screen.findByTestId('matrix-gap-row-1'));
    const rowFrame = screen.getByTestId('matrix-focus');
    expect(rowFrame.style.top).toBe(`${1 * MATRIX_CELL - 3}px`);
    expect(rowFrame.style.left).toBe('0px');

    await user.click(screen.getByTestId('matrix-gap-col-2'));
    const colFrame = screen.getByTestId('matrix-focus');
    expect(colFrame.style.left).toBe(`${2 * MATRIX_CELL - 3}px`);
    expect(colFrame.style.top).toBe('0px');
  });

  it('focuses, reviews and clears suspect links from the side panel', async () => {
    const scrollTo = vi.fn();
    const original = HTMLElement.prototype.scrollTo;
    HTMLElement.prototype.scrollTo = scrollTo;
    onTestFinished(() => {
      HTMLElement.prototype.scrollTo = original;
    });
    const user = userEvent.setup();
    renderPage();
    const item = await screen.findByTestId('matrix-suspect-0-1');
    expect(item).toHaveTextContent('REQ-COM-1 → TEST-B');

    await user.click(within(item).getByRole('button', { name: /REQ-COM-1/ }));
    expect(screen.getByTestId('matrix-focus').style.left).toBe(`${1 * MATRIX_CELL - 3}px`);
    expect(scrollTo).toHaveBeenCalled();

    await user.click(within(item).getByRole('button', { name: 'Review' }));
    expect(apiClient.listRequirementVersionsByProject).toHaveBeenCalledWith(5, 2);

    await user.click(within(item).getByRole('button', { name: 'Clear' }));
    expect(apiClient.clearTraceabilitySuspect).toHaveBeenCalledWith(2, 11, 'csrf-test');
  });

  it('keeps filters and sort in the URL', async () => {
    const user = userEvent.setup();
    renderPage();
    await screen.findByTestId('matrix-body');
    const suspect = screen.getByRole('switch', { name: /Suspect only/ });
    expect(suspect).toHaveAttribute('aria-checked', 'false');
    await user.click(suspect);
    await waitFor(() => expect(screen.getByTestId('location').textContent).toContain('mx_suspect=1'));
    expect(suspect).toHaveAttribute('aria-checked', 'true');
    expect(rowCodes()).toEqual(['REQ-COM-1']);
    await user.click(suspect);

    // Sorting by a verification column: linked rows first, no category bands.
    await user.click(screen.getByRole('button', { name: 'Sort rows by TEST-A' }));
    await waitFor(() => expect(screen.getByTestId('location').textContent).toContain('mx_sort=ver%3A10'));
    expect(rowCodes()[0]).toBe('REQ-PWR-2');
    expect(screen.queryByText('Comms')).not.toBeInTheDocument();
  });

  it('shows hover cards for requirements, verifications and cells', async () => {
    renderPage();
    fireEvent.mouseEnter(await screen.findByTestId('matrix-row-0'), { clientX: 40, clientY: 300 });
    expect(screen.getByRole('tooltip')).toHaveTextContent('Downlink');
    expect(screen.getByRole('tooltip')).toHaveTextContent('Comms · Accepted · draft');
    fireEvent.mouseLeave(screen.getByTestId('matrix-row-0'));

    fireEvent.mouseEnter(screen.getByTestId('matrix-col-2'), { clientX: 400, clientY: 100 });
    expect(screen.getByRole('tooltip')).toHaveTextContent('Orphan test');
    expect(screen.getByRole('tooltip')).toHaveTextContent('Covers 0 requirements');
    fireEvent.mouseLeave(screen.getByTestId('matrix-col-2'));

    fireEvent.mouseMove(screen.getByTestId('matrix-body'), { clientX: 1 * MATRIX_CELL + 5, clientY: 5 });
    const card = screen.getByRole('tooltip');
    expect(card).toHaveTextContent('REQ-COM-1 → TEST-B');
    expect(card).toHaveTextContent('Failed');
    expect(card).toHaveTextContent('Suspect. Requirement updated (since 2026-09-28).');
  });

  it('opens the requirement when a symbol is clicked, and exports', async () => {
    const user = userEvent.setup();
    renderPage();
    await user.click(await screen.findByRole('button', { name: 'Export Excel' }));
    expect(apiClient.downloadMatrixXlsx).toHaveBeenCalledWith(5);
    fireEvent.click(screen.getByTestId('matrix-body'), { clientX: 0 * MATRIX_CELL + 5, clientY: 2 * MATRIX_CELL + 5 });
    expect(await screen.findByText('requirement page')).toBeInTheDocument();
  });

  describe('filter toolbar (issue #361)', () => {
    const location = () => screen.getByTestId('location').textContent ?? '';
    const reqGroup = () => screen.getByRole('group', { name: 'Requirement status' });

    it('shows exact statuses only when selected, added from an accessible menu', async () => {
      const user = userEvent.setup();
      renderPage();
      await screen.findByTestId('matrix-body');
      // No status chips until a status is chosen; no badge nested in a toggle button.
      expect(within(reqGroup()).queryByRole('button', { name: /Remove requirement status filter/ })).toBeNull();
      expect(document.querySelectorAll('button[aria-pressed] span.border').length).toBe(0);

      const add = within(reqGroup()).getByRole('button', { name: 'Add requirement status filter' });
      expect(add).toHaveAttribute('aria-haspopup', 'menu');
      await user.click(add);
      const menu = screen.getByRole('menu', { name: 'Requirement status filters' });
      const items = within(menu).getAllByRole('menuitemcheckbox');
      expect(items.map((i) => i.textContent)).toEqual(['checkAccepted', 'checkDraft']);
      expect(items[0]).toHaveFocus();

      await user.click(items[0]);
      await user.click(within(menu).getByRole('menuitemcheckbox', { name: /Draft/ }));
      await waitFor(() => expect(location()).toContain('mx_rs=2%2C1'));
      expect(within(menu).getByRole('menuitemcheckbox', { name: /Accepted/ })).toHaveAttribute('aria-checked', 'true');
      expect(rowCodes().sort()).toEqual(['REQ-COM-1', 'REQ-PWR-1', 'REQ-PWR-2']);

      await user.keyboard('{Escape}');
      expect(screen.queryByRole('menu')).toBeNull();
      expect(add).toHaveFocus();

      // Each selected status is one removable chip.
      const accepted = within(reqGroup()).getByRole('button', { name: 'Remove requirement status filter: Accepted' });
      expect(accepted).not.toHaveAttribute('aria-pressed');
      await user.click(accepted);
      await waitFor(() => expect(location()).toContain('mx_rs=1'));
      expect(location()).not.toContain('mx_rs=2');
      expect(rowCodes().sort()).toEqual(['REQ-PWR-1', 'REQ-PWR-2']);
    });

    it('moves through the menu with the keyboard and closes on an outside click', async () => {
      const user = userEvent.setup();
      renderPage();
      await screen.findByTestId('matrix-body');
      const add = within(screen.getByRole('group', { name: 'Verification status' })).getByRole('button', {
        name: 'Add verification status filter',
      });
      add.focus();
      await user.keyboard('{ArrowDown}');
      const menu = screen.getByRole('menu', { name: 'Verification status filters' });
      const [failed, passed] = within(menu).getAllByRole('menuitemcheckbox');
      expect(failed).toHaveFocus();
      await user.keyboard('{ArrowDown}');
      expect(passed).toHaveFocus();
      await user.keyboard('{ArrowDown}');
      expect(failed).toHaveFocus();
      await user.keyboard('{End}');
      expect(passed).toHaveFocus();
      await user.keyboard('{Home}');
      expect(failed).toHaveFocus();
      await user.keyboard(' ');
      await waitFor(() => expect(location()).toContain('mx_vs=2'));
      await user.click(screen.getByTestId('matrix-stats'));
      expect(screen.queryByRole('menu')).toBeNull();
      expect(
        within(screen.getByRole('group', { name: 'Verification status' })).getByRole('button', {
          name: 'Remove verification status filter: Failed',
        }),
      ).toBeInTheDocument();
    });

    it('keeps status groups as neutral toggles and clears every filter at once', async () => {
      const user = userEvent.setup();
      renderPage('?mx_sort=ver%3A10');
      await screen.findByTestId('matrix-body');
      const clearAll = screen.getByRole('button', { name: /Clear all filters/ });
      expect(clearAll).toBeDisabled();
      const toolbar = screen.getByRole('region', { name: 'Matrix filters' });
      expect(within(toolbar).queryByRole('button', { name: 'Clear' })).toBeNull();

      const fail = within(screen.getByRole('group', { name: 'Status groups' })).getByRole('button', { name: /Fail \/ reject/ });
      expect(fail).toHaveAttribute('aria-pressed', 'false');
      await user.click(fail);
      await waitFor(() => expect(location()).toContain('mx_groups=fail'));
      expect(fail).toHaveAttribute('aria-pressed', 'true');
      await user.click(screen.getByRole('switch', { name: /Suspect only/ }));
      await waitFor(() => expect(location()).toContain('mx_suspect=1'));

      expect(clearAll).toBeEnabled();
      await user.click(clearAll);
      await waitFor(() => expect(location()).not.toContain('mx_groups'));
      expect(location()).not.toContain('mx_suspect');
      expect(location()).toContain('mx_sort=ver%3A10');
      expect(clearAll).toBeDisabled();
    });

    it('restores the toolbar from a bookmarked URL', async () => {
      // Filters that match nothing still show in the toolbar.
      renderPage('?mx_suspect=1&mx_groups=pass&mx_rs=2&mx_vs=2');
      await screen.findByRole('switch', { name: /Suspect only/ });
      expect(screen.getByRole('switch', { name: /Suspect only/ })).toHaveAttribute('aria-checked', 'true');
      expect(screen.getByRole('button', { name: /Pass \/ complete/ })).toHaveAttribute('aria-pressed', 'true');
      expect(screen.getByRole('button', { name: 'Remove requirement status filter: Accepted' })).toBeInTheDocument();
      expect(screen.getByRole('button', { name: 'Remove verification status filter: Failed' })).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /Clear all filters/ })).toBeEnabled();
    });
  });
});
