import { triggerDownload } from '@/utils/tableUtils';
import { fetchDownload, fetchJson, JSON_HEADERS } from './transport';
import type { Attachment, AttachmentEntityType, ProjectStorage } from './types';

function listUrl(projectId: number, entityType: AttachmentEntityType, entityId: number): string {
  const q = new URLSearchParams({ entity_type: entityType, entity_id: String(entityId) });
  return `/api/projects/${projectId}/attachments?${q}`;
}

/** Live attachments of one requirement or verification, oldest first. */
export async function listAttachments(
  projectId: number,
  entityType: AttachmentEntityType,
  entityId: number,
): Promise<Attachment[]> {
  return fetchJson<Attachment[]>(listUrl(projectId, entityType, entityId));
}

/** Upload a file (413 over the size limit or project quota, 415 for a disallowed type). */
export async function uploadAttachment(
  projectId: number,
  entityType: AttachmentEntityType,
  entityId: number,
  file: File,
  csrfToken: string,
): Promise<Attachment> {
  const body = new FormData();
  body.append('entity_type', entityType);
  body.append('entity_id', String(entityId));
  body.append('file', file);
  return fetchJson<Attachment>(`/api/projects/${projectId}/attachments`, {
    method: 'POST',
    headers: { 'X-CSRF-Token': csrfToken },
    body,
  });
}

export async function deleteAttachment(
  projectId: number,
  attachmentId: number,
  csrfToken: string,
): Promise<void> {
  await fetchJson<void>(`/api/projects/${projectId}/attachments/${attachmentId}`, {
    method: 'DELETE',
    headers: { 'X-CSRF-Token': csrfToken },
  });
}

async function download(path: string, fallbackName: string): Promise<void> {
  const { blob, filename } = await fetchDownload(path);
  triggerDownload(blob, filename ?? fallbackName);
}

export async function downloadAttachment(projectId: number, attachment: Attachment): Promise<void> {
  await download(
    `/api/projects/${projectId}/attachments/${attachment.id}/download`,
    attachment.filename,
  );
}

/** Files recorded in a baseline, including ones deleted since (`deleted: true`). */
export async function listBaselineAttachments(
  projectId: number,
  baselineId: number,
): Promise<Attachment[]> {
  return fetchJson<Attachment[]>(`/api/projects/${projectId}/baselines/${baselineId}/attachments`);
}

export async function downloadBaselineAttachment(
  projectId: number,
  baselineId: number,
  attachment: Attachment,
): Promise<void> {
  await download(
    `/api/projects/${projectId}/baselines/${baselineId}/attachments/${attachment.id}/download`,
    attachment.filename,
  );
}

/** Storage used by the project's attachments, its quota and the upload limits. */
export async function getProjectStorage(projectId: number): Promise<ProjectStorage> {
  return fetchJson<ProjectStorage>(`/api/projects/${projectId}/storage`);
}

/** Instance admins only. `null` goes back to the instance default. */
export async function setProjectStorageQuota(
  projectId: number,
  quotaMb: number | null,
  csrfToken: string,
): Promise<ProjectStorage> {
  return fetchJson<ProjectStorage>(`/api/projects/${projectId}/storage/quota`, {
    method: 'PUT',
    headers: { ...JSON_HEADERS, 'X-CSRF-Token': csrfToken },
    body: JSON.stringify({ quota_mb: quotaMb }),
  });
}
