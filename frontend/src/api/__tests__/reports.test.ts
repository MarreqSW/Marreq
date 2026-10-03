import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { CSRF, itSendsEachRequest, lastRequest, stubFetch } from '@/test/apiHarness';
import { triggerDownload } from '@/utils/tableUtils';
import {
  createReportTemplate,
  deleteReportTemplate,
  downloadReport,
  listReportTemplates,
  listReportTypes,
  updateReportTemplate,
  type ReportDefinition,
} from '../reports';

vi.mock('@/utils/tableUtils', () => ({ triggerDownload: vi.fn() }));

const definition = { report_type: 'vcd', sections: [{ key: 'matrix', enabled: true }] } as unknown as ReportDefinition;

describe('reports API (issue #354)', () => {
  beforeEach(() => vi.mocked(triggerDownload).mockClear());
  afterEach(() => vi.unstubAllGlobals());

  itSendsEachRequest([
    { name: 'listReportTypes', call: () => listReportTypes(5), method: 'GET', url: '/api/projects/5/reports/types' },
    {
      name: 'listReportTemplates',
      call: () => listReportTemplates(5),
      method: 'GET',
      url: '/api/projects/5/report_templates',
    },
    {
      name: 'createReportTemplate',
      call: () => createReportTemplate(5, { name: 'CDR', visibility: 'shared', definition }, CSRF),
      method: 'POST',
      url: '/api/projects/5/report_templates',
      body: { name: 'CDR', visibility: 'shared', definition },
    },
    {
      name: 'updateReportTemplate',
      call: () => updateReportTemplate(5, 9, { definition }, CSRF),
      method: 'PATCH',
      url: '/api/projects/5/report_templates/9',
      body: { definition },
    },
    {
      name: 'deleteReportTemplate',
      call: () => deleteReportTemplate(5, 9, CSRF),
      method: 'DELETE',
      url: '/api/projects/5/report_templates/9',
      response: { status: 204 },
    },
  ]);

  it('downloadReport posts the source and saves under the server filename', async () => {
    const fetchMock = stubFetch({
      text: '%PDF',
      headers: { 'Content-Disposition': 'attachment; filename="SAT-VCD-001-2026-10-03.pdf"' },
    });
    await downloadReport(5, 'vcd', 'pdf', { definition }, CSRF);
    const req = lastRequest(fetchMock);
    expect([req.method, req.url, req.headers['X-CSRF-Token']]).toEqual(['POST', '/api/projects/5/reports/vcd.pdf', CSRF]);
    expect(req.body).toEqual({ definition });
    expect(triggerDownload).toHaveBeenCalledWith(expect.any(Blob), 'SAT-VCD-001-2026-10-03.pdf');
  });

  it('downloadReport falls back to a generic name and surfaces errors', async () => {
    stubFetch({ text: 'PK' });
    await downloadReport(5, 'coverage', 'odt', { template_id: 3 }, CSRF);
    expect(triggerDownload).toHaveBeenCalledWith(expect.any(Blob), 'coverage-report.odt');

    stubFetch({ status: 400, json: { message: "section 'toc' is listed twice" } });
    await expect(downloadReport(5, 'vcd', 'pdf', {}, CSRF)).rejects.toThrow('listed twice');
  });
});
