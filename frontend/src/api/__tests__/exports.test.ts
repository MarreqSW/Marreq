import { afterEach, describe, expect, it, vi } from 'vitest';
import { downloadRequirementsXlsx, downloadVerificationsXlsx } from '../exports';

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
