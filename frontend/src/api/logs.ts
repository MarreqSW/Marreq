import type {
  AdminLogListParams,
  AdminLogListResponse,
  AdminLogStats,
  AdminLogStatsParams,
} from './types';
import { fetchBlob, fetchDownload, fetchJson, JSON_HEADERS } from './transport';
import { triggerDownload } from '@/utils/tableUtils';

function logsQuery(params: AdminLogListParams & { top?: number }): string {
  const q = new URLSearchParams();
  const set = (key: string, value: string | number | undefined) => {
    if (value === undefined || value === '') return;
    q.set(key, String(value));
  };
  set('entity_type', params.entity_type);
  set('entity_id', params.entity_id);
  set('user_id', params.user_id);
  set('action_type', params.action_type);
  set('project_id', params.project_id);
  set('since', params.since);
  set('until', params.until);
  set('limit', params.limit);
  set('offset', params.offset);
  set('top', params.top);
  const s = q.toString();
  return s ? `?${s}` : '';
}

export async function listAdminLogs(
  params: AdminLogListParams = {},
): Promise<AdminLogListResponse> {
  return fetchJson<AdminLogListResponse>(`/api/admin/logs${logsQuery(params)}`);
}

export async function downloadAdminLogsJson(
  params: AdminLogListParams = {},
): Promise<void> {
  const blob = await fetchBlob(`/api/admin/logs/export.json${logsQuery(params)}`);
  triggerDownload(blob, 'audit-logs.json');
}

export async function cleanupAdminLogs(
  days: number,
  csrfToken: string,
): Promise<{ deleted: number }> {
  return fetchJson<{ deleted: number }>('/api/admin/logs/cleanup', {
    method: 'POST',
    headers: { ...JSON_HEADERS, 'X-CSRF-Token': csrfToken },
    body: JSON.stringify({ days }),
  });
}

/** Activity summary (events per day, top actions, top users) for site administrators. */
export async function getAdminLogStats(params: AdminLogStatsParams = {}): Promise<AdminLogStats> {
  return fetchJson<AdminLogStats>(`/api/admin/logs/stats${logsQuery(params)}`);
}

/** What a backup with attachment files would contain (issue #341). */
export type BackupInfo = {
  /** False when the server has no attachment storage (backups hold the database only). */
  attachments_available: boolean;
  attachment_files: number;
  attachment_bytes: number;
};

export async function getBackupInfo(): Promise<BackupInfo> {
  return fetchJson<BackupInfo>('/api/admin/backup/info');
}

/**
 * Runs a whole-instance backup on the server (admin only, self-hosted) and saves the
 * `.tar.gz` with the SQL dump and, unless `attachments` is false, the attachment
 * files. Resolves with the downloaded filename.
 */
export async function downloadDatabaseBackup(
  csrfToken: string,
  { attachments = true }: { attachments?: boolean } = {},
): Promise<string> {
  const query = attachments ? '' : '?attachments=false';
  const { blob, filename } = await fetchDownload(`/api/admin/backup${query}`, {
    method: 'POST',
    headers: { 'X-CSRF-Token': csrfToken },
  });
  const name = filename ?? 'marreq-backup.tar.gz';
  triggerDownload(blob, name);
  return name;
}
