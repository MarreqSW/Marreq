import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { CSRF, itSendsEachRequest, lastRequest, stubFetch } from '@/test/apiHarness';
import { triggerDownload } from '@/utils/tableUtils';
import {
  deleteAttachment,
  downloadAttachment,
  downloadBaselineAttachment,
  getProjectStorage,
  listAttachments,
  listBaselineAttachments,
  setProjectStorageQuota,
  uploadAttachment,
} from '../attachments';
import type { Attachment } from '../types';

vi.mock('@/utils/tableUtils', () => ({ triggerDownload: vi.fn() }));

const attachment = { id: 12, filename: 'spec sheet.pdf' } as Attachment;

describe('attachments API', () => {
  beforeEach(() => vi.mocked(triggerDownload).mockClear());
  afterEach(() => vi.unstubAllGlobals());

  itSendsEachRequest([
    {
      name: 'listAttachments filters by entity',
      call: () => listAttachments(5, 'requirement', 42),
      method: 'GET',
      url: '/api/projects/5/attachments?entity_type=requirement&entity_id=42',
    },
    {
      name: 'uploadAttachment sends a multipart form',
      call: () => uploadAttachment(5, 'verification', 77, new File(['%PDF'], 'report.pdf'), CSRF),
      method: 'POST',
      url: '/api/projects/5/attachments',
      body: { entity_type: 'verification', entity_id: '77', file: 'file:report.pdf' },
    },
    {
      name: 'deleteAttachment',
      call: () => deleteAttachment(5, 12, CSRF),
      method: 'DELETE',
      url: '/api/projects/5/attachments/12',
    },
    {
      name: 'listBaselineAttachments',
      call: () => listBaselineAttachments(5, 3),
      method: 'GET',
      url: '/api/projects/5/baselines/3/attachments',
    },
    {
      name: 'getProjectStorage',
      call: () => getProjectStorage(5),
      method: 'GET',
      url: '/api/projects/5/storage',
    },
    {
      name: 'setProjectStorageQuota',
      call: () => setProjectStorageQuota(5, 750, CSRF),
      method: 'PUT',
      url: '/api/projects/5/storage/quota',
      body: { quota_mb: 750 },
    },
    {
      name: 'setProjectStorageQuota resets to the default with null',
      call: () => setProjectStorageQuota(5, null, CSRF),
      method: 'PUT',
      url: '/api/projects/5/storage/quota',
      body: { quota_mb: null },
    },
  ]);

  it('downloadAttachment saves the file under the server-suggested name', async () => {
    const fetchMock = stubFetch({
      text: '%PDF',
      headers: { 'Content-Disposition': 'attachment; filename="server-name.pdf"' },
    });
    await downloadAttachment(5, attachment);
    expect(lastRequest(fetchMock).url).toBe('/api/projects/5/attachments/12/download');
    expect(triggerDownload).toHaveBeenCalledWith(expect.any(Blob), 'server-name.pdf');
  });

  it('downloadBaselineAttachment falls back to the recorded filename', async () => {
    const fetchMock = stubFetch({ text: '%PDF' });
    await downloadBaselineAttachment(5, 3, attachment);
    expect(lastRequest(fetchMock).url).toBe('/api/projects/5/baselines/3/attachments/12/download');
    expect(triggerDownload).toHaveBeenCalledWith(expect.any(Blob), 'spec sheet.pdf');
  });

  it('a failed download does not save anything', async () => {
    stubFetch({ status: 404, json: { message: 'attachment not found' } });
    await expect(downloadAttachment(5, attachment)).rejects.toThrow('attachment not found');
    expect(triggerDownload).not.toHaveBeenCalled();
  });
});
