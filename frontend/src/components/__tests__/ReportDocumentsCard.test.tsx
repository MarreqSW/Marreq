import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as apiClient from '@/api/client';
import { reportTemplate, reportTypes } from '@/test/reportFixtures';
import ReportDocumentsCard from '../reports/ReportDocumentsCard';

vi.mock('@/api/client');

function renderCard() {
  return render(
    <MemoryRouter>
      <ReportDocumentsCard projectId={5} basePath="/sat" csrfToken="csrf" />
    </MemoryRouter>,
  );
}

describe('ReportDocumentsCard', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(apiClient.listReportTypes).mockResolvedValue(reportTypes());
    vi.mocked(apiClient.listReportTemplates).mockResolvedValue([
      reportTemplate(),
      reportTemplate({ id: 9, name: 'Coverage short', report_type: 'coverage', visibility: 'shared' }),
    ]);
    vi.mocked(apiClient.downloadReport).mockResolvedValue(undefined);
  });

  it('generates the default VCD as PDF and links to the builder', async () => {
    renderCard();
    await userEvent.click(await screen.findByRole('button', { name: 'Generate' }));
    expect(apiClient.downloadReport).toHaveBeenCalledWith(5, 'vcd', 'pdf', {}, 'csrf');
    expect(screen.getByRole('link', { name: 'Customize…' })).toHaveAttribute('href', '/sat/reports/builder?type=vcd');
  });

  it('lists the templates of the chosen type and generates from one as ODT', async () => {
    renderCard();
    const template = await screen.findByRole('combobox', { name: 'Template' });
    expect(screen.getAllByRole('option').map((o) => o.textContent)).toContain('CDR VCD');
    await userEvent.selectOptions(screen.getByRole('combobox', { name: 'Report' }), 'coverage');
    expect([...template.querySelectorAll('option')].map((o) => o.textContent)).toEqual([
      'Default',
      'Coverage short (shared)',
    ]);
    await userEvent.selectOptions(template, '9');
    await userEvent.click(screen.getByRole('radio', { name: 'ODT' }));
    expect(screen.getByRole('link', { name: 'Customize…' })).toHaveAttribute(
      'href',
      '/sat/reports/builder?type=coverage&template=9',
    );
    await userEvent.click(screen.getByRole('button', { name: 'Generate' }));
    expect(apiClient.downloadReport).toHaveBeenCalledWith(5, 'coverage', 'odt', { template_id: 9 }, 'csrf');
  });

  it('shows errors', async () => {
    vi.mocked(apiClient.downloadReport).mockRejectedValue(new Error('report layout failed'));
    renderCard();
    await userEvent.click(await screen.findByRole('button', { name: 'Generate' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('report layout failed');
  });
});
