import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Outlet, Route, Routes, useLocation } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as apiClient from '@/api/client';
import type { ReportDefinition } from '@/api/reports';
import { reportTemplate, reportTypes } from '@/test/reportFixtures';
import ReportBuilderPage from '../ReportBuilderPage';

vi.mock('@/api/client');
vi.mock('@/context/DashboardContext', () => ({
  useDashboard: () => ({ csrfToken: 'csrf', dashboard: { projects: [{ id: 5, name: 'Satellite Demo' }] } }),
}));

function Where() {
  const { search } = useLocation();
  return <output data-testid="where">{search}</output>;
}

function renderAt(search = '?type=vcd') {
  return render(
    <MemoryRouter initialEntries={[`/sat/reports/builder${search}`]}>
      <Routes>
        <Route
          path="/sat"
          element={
            <>
              <Outlet context={{ projectId: 5, basePath: '/sat', globalSearch: '', setGlobalSearch: vi.fn() }} />
              <Where />
            </>
          }
        >
          <Route path="reports/builder" element={<ReportBuilderPage />} />
        </Route>
      </Routes>
    </MemoryRouter>,
  );
}

const sectionTitles = () =>
  within(screen.getByRole('list', { name: 'Report sections' }))
    .getAllByRole('listitem')
    .map((li) => li.querySelector('p')?.textContent?.replace(/^\d+\./, '') ?? '');

const sentDefinition = (): ReportDefinition =>
  (vi.mocked(apiClient.downloadReport).mock.calls.at(-1)?.[3] as { definition: ReportDefinition }).definition;

describe('ReportBuilderPage', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(apiClient.listReportTypes).mockResolvedValue(reportTypes());
    vi.mocked(apiClient.listReportTemplates).mockResolvedValue([reportTemplate()]);
    vi.mocked(apiClient.downloadReport).mockResolvedValue(undefined);
    vi.spyOn(window, 'confirm').mockReturnValue(true);
  });

  it('reorders, toggles and configures sections, and downloads what is on screen', async () => {
    renderAt();
    await screen.findByText('Verification control matrix');
    expect(sectionTitles()).toEqual(['Cover page', 'Contents', 'Verification control matrix']);

    await userEvent.click(screen.getByRole('button', { name: 'Move Verification control matrix up' }));
    await userEvent.click(screen.getByRole('button', { name: 'Move Verification control matrix up' }));
    expect(sectionTitles()).toEqual(['Verification control matrix', 'Cover page', 'Contents']);
    expect(screen.getByRole('button', { name: 'Move Verification control matrix up' })).toBeDisabled();
    expect(screen.getByText('Unsaved changes.')).toBeInTheDocument();

    await userEvent.click(screen.getByRole('checkbox', { name: 'Include Contents' }));
    await userEvent.click(screen.getByRole('button', { name: 'Options of Verification control matrix' }));
    await userEvent.selectOptions(screen.getByRole('combobox', { name: 'Group by' }), 'category');
    // The last selected column cannot be unticked.
    await userEvent.click(screen.getByRole('checkbox', { name: 'Close-out' }));
    expect(screen.getByRole('checkbox', { name: 'Req. ID' })).toBeDisabled();

    await userEvent.clear(screen.getByPlaceholderText('e.g. Company internal'));
    await userEvent.type(screen.getByPlaceholderText('e.g. Company internal'), 'Confidential');
    await userEvent.click(screen.getByRole('button', { name: 'Download ODT' }));

    expect(apiClient.downloadReport).toHaveBeenCalledWith(5, 'vcd', 'odt', expect.anything(), 'csrf');
    const def = sentDefinition();
    expect(def.sections).toEqual([
      { key: 'matrix', enabled: true, options: { group_by: 'category', columns: ['id'] } },
      { key: 'cover', enabled: true },
      { key: 'toc', enabled: false },
    ]);
    expect(def.document.classification).toBe('Confidential');
  });

  it('reorders by drag and drop', async () => {
    renderAt();
    await screen.findByText('Verification control matrix');
    const items = within(screen.getByRole('list', { name: 'Report sections' })).getAllByRole('listitem');
    const { fireEvent } = await import('@testing-library/react');
    const dataTransfer = { effectAllowed: '', setData: vi.fn(), getData: vi.fn() };
    fireEvent.dragStart(items[0], { dataTransfer });
    fireEvent.dragOver(items[2], { dataTransfer });
    fireEvent.drop(items[2], { dataTransfer });
    expect(sectionTitles()).toEqual(['Contents', 'Verification control matrix', 'Cover page']);
  });

  it('opens a saved template, saves changes to it and can reset to the default', async () => {
    vi.mocked(apiClient.updateReportTemplate).mockResolvedValue(reportTemplate());
    renderAt('?type=vcd&template=7');
    await screen.findByText('Verification control matrix');
    expect(sectionTitles()).toEqual(['Verification control matrix', 'Cover page', 'Contents']);
    expect(screen.getByRole('checkbox', { name: 'Include Contents' })).not.toBeChecked();

    const save = screen.getByRole('button', { name: 'Save' });
    expect(save).toBeDisabled();
    await userEvent.click(screen.getByRole('checkbox', { name: 'Include Contents' }));
    await userEvent.click(save);
    expect(apiClient.updateReportTemplate).toHaveBeenCalledWith(
      5,
      7,
      { definition: expect.objectContaining({ report_type: 'vcd' }) },
      'csrf',
    );
    expect(await screen.findByText('Saved "CDR VCD".')).toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: 'Reset to default' }));
    expect(sectionTitles()).toEqual(['Cover page', 'Contents', 'Verification control matrix']);
  });

  it('saves a copy with Save as… and selects it', async () => {
    vi.mocked(apiClient.createReportTemplate).mockResolvedValue(reportTemplate({ id: 8, name: 'Mine' }));
    renderAt();
    await screen.findByText('Verification control matrix');
    await userEvent.click(screen.getByRole('button', { name: 'Save as…' }));
    const dialog = screen.getByRole('dialog');
    await userEvent.type(within(dialog).getByRole('textbox'), 'Mine');
    await userEvent.click(within(dialog).getByRole('checkbox', { name: 'Share with all project members' }));
    await userEvent.click(within(dialog).getByRole('button', { name: 'Save' }));
    expect(apiClient.createReportTemplate).toHaveBeenCalledWith(
      5,
      { name: 'Mine', visibility: 'shared', definition: expect.objectContaining({ report_type: 'vcd' }) },
      'csrf',
    );
    await waitFor(() => expect(screen.getByTestId('where')).toHaveTextContent('?type=vcd&template=8'));
  });

  it("is read-only for someone else's template but still lets you download and copy it", async () => {
    vi.mocked(apiClient.listReportTemplates).mockResolvedValue([
      reportTemplate({ can_edit: false, owner_name: 'Bob', visibility: 'shared' }),
    ]);
    renderAt('?type=vcd&template=7');
    await screen.findByText(/This template belongs to Bob/);
    expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled();
    expect(screen.queryByRole('button', { name: 'Delete' })).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Download PDF' }));
    expect(apiClient.downloadReport).toHaveBeenCalledWith(5, 'vcd', 'pdf', expect.anything(), 'csrf');
  });

  it('deletes a template after confirming and asks before discarding changes', async () => {
    vi.mocked(apiClient.deleteReportTemplate).mockResolvedValue(undefined);
    renderAt('?type=vcd&template=7');
    await screen.findByText('Verification control matrix');
    await userEvent.click(screen.getByRole('button', { name: 'Delete' }));
    expect(apiClient.deleteReportTemplate).toHaveBeenCalledWith(5, 7, 'csrf');
    await waitFor(() => expect(screen.getByTestId('where')).toHaveTextContent('?type=vcd'));

    await userEvent.click(screen.getByRole('checkbox', { name: 'Include Contents' }));
    vi.mocked(window.confirm).mockReturnValue(false);
    await userEvent.selectOptions(screen.getByRole('combobox', { name: 'Report' }), 'coverage');
    expect(screen.getByTestId('where')).toHaveTextContent('?type=vcd');
    vi.mocked(window.confirm).mockReturnValue(true);
    await userEvent.selectOptions(screen.getByRole('combobox', { name: 'Report' }), 'coverage');
    expect(screen.getByTestId('where')).toHaveTextContent('?type=coverage');
  });

  it('shows generation errors', async () => {
    vi.mocked(apiClient.downloadReport).mockRejectedValue(new Error("section 'toc' is listed twice"));
    renderAt();
    await userEvent.click(await screen.findByRole('button', { name: 'Download PDF' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('listed twice');
  });
});
