import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  downloadBaselineReqif,
  downloadMatrixLinksXlsx,
  downloadMatrixXlsx,
  downloadProjectBundleJson,
  downloadProjectReportPdf,
  downloadRequirementsPdf,
  downloadRequirementsReqif,
  downloadRequirementsXlsx,
  downloadVerificationsXlsx,
} from '../exports';

function stubDownloadEnvironment() {
  const anchor = { href: '', download: '', click: vi.fn() };
  vi.spyOn(document, 'createElement').mockReturnValue(anchor as unknown as HTMLAnchorElement);
  vi.spyOn(URL, 'createObjectURL').mockReturnValue('blob:workbook');
  vi.spyOn(URL, 'revokeObjectURL').mockImplementation(() => {});
  return anchor;
}

function stubFetchOk(blob: Blob) {
  const fetchMock = vi.fn().mockResolvedValue({
    ok: true,
    status: 200,
    statusText: 'OK',
    blob: async () => blob,
  });
  vi.stubGlobal('fetch', fetchMock);
  return fetchMock;
}

describe('workbook downloads', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it('downloads the requirements workbook for a project', async () => {
    const anchor = stubDownloadEnvironment();
    const fetchMock = stubFetchOk(new Blob(['workbook']));

    await downloadRequirementsXlsx(7);

    expect(fetchMock).toHaveBeenCalledWith(
      '/api/projects/7/exports/requirements.xlsx',
      expect.objectContaining({ credentials: 'same-origin' }),
    );
    expect(anchor.download).toBe('requirements-project-7.xlsx');
    expect(anchor.click).toHaveBeenCalled();
  });

  it('downloads the verifications workbook for a project', async () => {
    const anchor = stubDownloadEnvironment();
    const fetchMock = stubFetchOk(new Blob(['workbook']));

    await downloadVerificationsXlsx(7);

    expect(fetchMock).toHaveBeenCalledWith(
      '/api/projects/7/exports/verifications.xlsx',
      expect.objectContaining({ credentials: 'same-origin' }),
    );
    expect(anchor.download).toBe('verifications-project-7.xlsx');
  });

  it.each([
    {
      name: 'the traceability matrix workbook',
      download: downloadMatrixXlsx,
      path: '/api/projects/7/exports/matrix.xlsx',
      filename: 'matrix-project-7.xlsx',
    },
    {
      name: 'the matrix links workbook',
      download: downloadMatrixLinksXlsx,
      path: '/api/projects/7/exports/matrix-links.xlsx',
      filename: 'matrix-links-project-7.xlsx',
    },
    {
      name: 'the requirements PDF',
      download: downloadRequirementsPdf,
      path: '/api/projects/7/exports/requirements.pdf',
      filename: 'requirements-project-7.pdf',
    },
    {
      name: 'the project report PDF',
      download: downloadProjectReportPdf,
      path: '/api/projects/7/exports/report.pdf',
      filename: 'report-project-7.pdf',
    },
    {
      name: 'the project ReqIF file',
      download: downloadRequirementsReqif,
      path: '/api/projects/7/exports/requirements.reqif',
      filename: 'requirements-project-7.reqif',
    },
    {
      name: 'the project JSON bundle',
      download: downloadProjectBundleJson,
      path: '/api/projects/7/exports/bundle.json',
      filename: 'project-7-bundle.json',
    },
  ])('downloads $name for a project', async ({ download, path, filename }) => {
    const anchor = stubDownloadEnvironment();
    const fetchMock = stubFetchOk(new Blob(['document']));

    await download(7);

    expect(fetchMock).toHaveBeenCalledWith(
      path,
      expect.objectContaining({ credentials: 'same-origin' }),
    );
    expect(anchor.download).toBe(filename);
    expect(anchor.click).toHaveBeenCalled();
  });

  it('downloads a baseline ReqIF file for a project', async () => {
    const anchor = stubDownloadEnvironment();
    const fetchMock = stubFetchOk(new Blob(['reqif']));

    await downloadBaselineReqif(7, 10);

    expect(fetchMock).toHaveBeenCalledWith(
      '/api/projects/7/exports/baselines/10.reqif',
      expect.objectContaining({ credentials: 'same-origin' }),
    );
    expect(anchor.download).toBe('baseline-10-project-7.reqif');
    expect(anchor.click).toHaveBeenCalled();
  });

  it('surfaces the server error message and downloads nothing', async () => {
    const anchor = stubDownloadEnvironment();
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: false,
        status: 403,
        statusText: 'Forbidden',
        text: async () => JSON.stringify({ message: 'You lack permission' }),
      }),
    );

    await expect(downloadRequirementsXlsx(7)).rejects.toThrow('You lack permission');
    expect(anchor.click).not.toHaveBeenCalled();
  });
});
