import { fetchBlob, fetchDownload } from './transport';
import { triggerDownload } from '@/utils/tableUtils';
import { dsmQueryString, type DsmParams } from '@/utils/dsm';

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

/** Downloads the traceability matrix workbook (requirements as rows, verifications as columns). */
export async function downloadMatrixXlsx(projectId: number): Promise<void> {
  const blob = await fetchBlob(`/api/projects/${projectId}/exports/matrix.xlsx`);
  triggerDownload(blob, `matrix-project-${projectId}.xlsx`);
}

/** Downloads the dependency structure matrix (same filters as the DSM view). */
export async function downloadDsmXlsx(projectId: number, params: DsmParams): Promise<void> {
  const blob = await fetchBlob(
    `/api/projects/${projectId}/exports/dsm.xlsx${dsmQueryString(params)}`,
  );
  triggerDownload(blob, `dsm-project-${projectId}.xlsx`);
}

/** Downloads matrix links as two code columns, matching Import → Matrix links. */
export async function downloadMatrixLinksXlsx(projectId: number): Promise<void> {
  const blob = await fetchBlob(`/api/projects/${projectId}/exports/matrix-links.xlsx`);
  triggerDownload(blob, `matrix-links-project-${projectId}.xlsx`);
}

/** Downloads the requirements table as PDF. */
export async function downloadRequirementsPdf(projectId: number): Promise<void> {
  const blob = await fetchBlob(`/api/projects/${projectId}/exports/requirements.pdf`);
  triggerDownload(blob, `requirements-project-${projectId}.pdf`);
}

/** Downloads the project summary report (totals, coverage, status breakdowns) as PDF. */
export async function downloadProjectReportPdf(projectId: number): Promise<void> {
  const blob = await fetchBlob(`/api/projects/${projectId}/exports/report.pdf`);
  triggerDownload(blob, `report-project-${projectId}.pdf`);
}

/** Downloads the current project requirements as ReqIF 1.2 XML. */
export async function downloadRequirementsReqif(projectId: number): Promise<void> {
  const blob = await fetchBlob(`/api/projects/${projectId}/exports/requirements.reqif`);
  triggerDownload(blob, `requirements-project-${projectId}.reqif`);
}

/** Downloads an immutable baseline snapshot as ReqIF 1.2 XML. */
export async function downloadBaselineReqif(
  projectId: number,
  baselineId: number,
): Promise<void> {
  const blob = await fetchBlob(
    `/api/projects/${projectId}/exports/baselines/${baselineId}.reqif`,
  );
  triggerDownload(blob, `baseline-${baselineId}-project-${projectId}.reqif`);
}

/** Downloads the current project requirements with their attachment files as a ReqIFZ archive. */
export async function downloadRequirementsReqifz(projectId: number): Promise<void> {
  const blob = await fetchBlob(`/api/projects/${projectId}/exports/requirements.reqifz`);
  triggerDownload(blob, `requirements-project-${projectId}.reqifz`);
}

/** Downloads a baseline snapshot with the files it recorded as a ReqIFZ archive. */
export async function downloadBaselineReqifz(
  projectId: number,
  baselineId: number,
): Promise<void> {
  const blob = await fetchBlob(
    `/api/projects/${projectId}/exports/baselines/${baselineId}.reqifz`,
  );
  triggerDownload(blob, `baseline-${baselineId}-project-${projectId}.reqifz`);
}

/** Downloads a JSON project bundle (catalog, current requirements, tests, matrix, comments). */
export async function downloadProjectBundleJson(projectId: number): Promise<void> {
  const blob = await fetchBlob(`/api/projects/${projectId}/exports/bundle.json`);
  triggerDownload(blob, `project-${projectId}-bundle.json`);
}

/**
 * Downloads the project bundle with the attachment files of its requirements and
 * verifications as a ZIP archive (issue #341).
 */
export async function downloadProjectBundleZip(projectId: number): Promise<void> {
  const { blob, filename } = await fetchDownload(`/api/projects/${projectId}/exports/bundle.zip`);
  triggerDownload(blob, filename ?? `project-${projectId}-bundle.zip`);
}
