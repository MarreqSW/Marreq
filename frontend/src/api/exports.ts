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

/** Downloads the traceability matrix workbook (requirements as rows, verifications as columns). */
export async function downloadMatrixXlsx(projectId: number): Promise<void> {
  const blob = await fetchBlob(`/api/projects/${projectId}/exports/matrix.xlsx`);
  triggerDownload(blob, `matrix-project-${projectId}.xlsx`);
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

/** Downloads a JSON project bundle (catalog, current requirements, tests, matrix, comments). */
export async function downloadProjectBundleJson(projectId: number): Promise<void> {
  const blob = await fetchBlob(`/api/projects/${projectId}/exports/bundle.json`);
  triggerDownload(blob, `project-${projectId}-bundle.json`);
}
