import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as apiClient from '@/api/client';
import { ApiError } from '@/api/transport';
import type { Attachment, ProjectStorage } from '@/api/types';
import AttachmentsPanel, { precheckFile } from '../AttachmentsPanel';

vi.mock('@/api/client', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api/client')>();
  return {
    ...actual,
    listAttachments: vi.fn(),
    getProjectStorage: vi.fn(),
    uploadAttachment: vi.fn(),
    deleteAttachment: vi.fn(),
    downloadAttachment: vi.fn(),
  };
});

const storage: ProjectStorage = {
  used_bytes: 300 * 1024,
  quota_bytes: 500 * 1024 * 1024,
  quota_is_default: true,
  default_quota_bytes: 500 * 1024 * 1024,
  retained_by_baselines_bytes: 0,
  max_file_bytes: 10 * 1024 * 1024,
  allowed_extensions: ['pdf', 'png', 'csv'],
};

const attachment = (id: number, filename: string): Attachment => ({
  id,
  entity_type: 'requirement',
  entity_id: 7,
  filename,
  content_type: 'application/pdf',
  size_bytes: 300 * 1024,
  uploaded_by: 2,
  uploaded_by_name: 'Dr Smith',
  created_at: '2026-09-30T10:00:00',
  deleted: false,
});

function renderPanel(canEdit = true) {
  render(
    <AttachmentsPanel projectId={5} entityType="requirement" entityId={7} canEdit={canEdit} csrfToken="csrf" />,
  );
}

beforeEach(() => {
  vi.mocked(apiClient.listAttachments).mockReset().mockResolvedValue([attachment(1, 'spec.pdf')]);
  vi.mocked(apiClient.getProjectStorage).mockReset().mockResolvedValue(storage);
  vi.mocked(apiClient.uploadAttachment).mockReset();
  vi.mocked(apiClient.deleteAttachment).mockReset();
  vi.mocked(apiClient.downloadAttachment).mockReset().mockResolvedValue();
});

describe('AttachmentsPanel', () => {
  it('lists files with size, uploader and storage use, and downloads on click', async () => {
    const user = userEvent.setup();
    renderPanel();
    const file = await screen.findByRole('button', { name: 'spec.pdf' });
    expect(screen.getByText(/300 KB · Dr Smith/)).toBeInTheDocument();
    expect(screen.getByText('Project storage: 300 KB of 500 MB used')).toBeInTheDocument();
    expect(apiClient.listAttachments).toHaveBeenCalledWith(5, 'requirement', 7);
    await user.click(file);
    expect(apiClient.downloadAttachment).toHaveBeenCalledWith(5, attachment(1, 'spec.pdf'));
  });

  it('uploads chosen files and shows server errors', async () => {
    vi.mocked(apiClient.uploadAttachment)
      .mockResolvedValueOnce(attachment(2, 'plot.png'))
      .mockRejectedValueOnce(new ApiError(413, 'Project storage limit reached'));
    const user = userEvent.setup();
    renderPanel();
    await screen.findByRole('button', { name: 'spec.pdf' });
    const input = screen.getByLabelText('Upload attachments');
    expect(input).toHaveAttribute('accept', '.pdf,.png,.csv');
    await user.upload(input, [
      new File(['png'], 'plot.png', { type: 'image/png' }),
      new File(['a,b'], 'data.csv', { type: 'text/csv' }),
    ]);
    expect(await screen.findByRole('button', { name: 'plot.png' })).toBeInTheDocument();
    expect(apiClient.uploadAttachment).toHaveBeenCalledWith(5, 'requirement', 7, expect.any(File), 'csrf');
    expect(await screen.findByRole('alert')).toHaveTextContent('data.csv: Project storage limit reached');
  });

  it('rejects disallowed types before uploading', async () => {
    const user = userEvent.setup({ applyAccept: false });
    renderPanel();
    await screen.findByRole('button', { name: 'spec.pdf' });
    await user.upload(screen.getByLabelText('Upload attachments'), new File(['<html>'], 'page.html'));
    expect(await screen.findByRole('alert')).toHaveTextContent('.html files are not allowed');
    expect(apiClient.uploadAttachment).not.toHaveBeenCalled();
  });

  it('deletes after confirmation', async () => {
    vi.spyOn(window, 'confirm').mockReturnValue(true);
    vi.mocked(apiClient.deleteAttachment).mockResolvedValue();
    const user = userEvent.setup();
    renderPanel();
    await user.click(await screen.findByRole('button', { name: 'Delete spec.pdf' }));
    await waitFor(() => expect(screen.queryByRole('button', { name: 'spec.pdf' })).not.toBeInTheDocument());
    expect(apiClient.deleteAttachment).toHaveBeenCalledWith(5, 1, 'csrf');
    expect(screen.getByText('No files attached.')).toBeInTheDocument();
  });

  it('is read-only without edit permission', async () => {
    renderPanel(false);
    const panel = await screen.findByRole('region', { name: 'Attachments' });
    await within(panel).findByRole('button', { name: 'spec.pdf' });
    expect(screen.queryByRole('button', { name: /Add files/ })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Delete spec.pdf' })).not.toBeInTheDocument();
    expect(screen.queryByLabelText('Upload attachments')).not.toBeInTheDocument();
  });
});

describe('precheckFile', () => {
  it('checks size, emptiness and extension against the limits', () => {
    expect(precheckFile(new File(['x'], 'a.pdf'), storage)).toBeNull();
    expect(precheckFile(new File([], 'a.pdf'), storage)).toMatch(/empty/);
    const big = new File(['x'], 'big.pdf');
    Object.defineProperty(big, 'size', { value: 11 * 1024 * 1024 });
    expect(precheckFile(big, storage)).toMatch(/at most 10 MB/);
    expect(precheckFile(new File(['x'], 'README'), storage)).toMatch(/without an extension/);
    expect(precheckFile(new File(['x'], 'a.exe'), null)).toBeNull();
  });
});
