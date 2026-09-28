import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes, useLocation, useOutletContext } from 'react-router-dom';
import { beforeEach, describe, expect, it, onTestFinished, vi } from 'vitest';
import * as apiClient from '@/api/client';
import type { MatrixLink, Requirement, Verification } from '@/api/types';
import { MATRIX_CELL } from '@/components/matrix/MatrixGrid';
import MatrixPage from '../MatrixPage';

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
              <MatrixPage />
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

describe('MatrixPage (DSM-style grid)', () => {
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
    await user.click(screen.getByRole('button', { name: /Suspect only/ }));
    await waitFor(() => expect(screen.getByTestId('location').textContent).toContain('mx_suspect=1'));
    expect(rowCodes()).toEqual(['REQ-COM-1']);
    await user.click(screen.getByRole('button', { name: /Suspect only/ }));

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
});
