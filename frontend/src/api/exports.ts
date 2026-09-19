import { fetchBlob } from './transport';
import { triggerDownload } from '@/utils/tableUtils';

/** Downloads the project requirements workbook (all requirements, not just the filtered rows). */
export async function downloadRequirementsXlsx(projectId: number): Promise<void> {
  const blob = await fetchBlob(`/api/projects/${projectId}/exports/requirements.xlsx`);
  triggerDownload(blob, `requirements-project-${projectId}.xlsx`);
}

/** Downloads the project verifications workbook (all verifications, not just the filtered rows). */
export async function downloadVerificationsXlsx(projectId: number): Promise<void> {
  const blob = await fetchBlob(`/api/projects/${projectId}/exports/verifications.xlsx`);
  triggerDownload(blob, `verifications-project-${projectId}.xlsx`);
}
