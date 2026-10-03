import { triggerDownload } from '@/utils/tableUtils';
import { fetchDownload, fetchJson, JSON_HEADERS } from './transport';

/** Report document types (issue #354). */
export type ReportTypeKey = 'vcd' | 'coverage';
export type ReportFormat = 'pdf' | 'odt';

export interface ReportChoice {
  value: string;
  label: string;
}

/** Option schema of a section; `kind` decides the control. */
export type ReportOptionSpec = {
  key: string;
  label: string;
  default: unknown;
} & (
  | { kind: 'select'; choices: ReportChoice[] }
  | { kind: 'multi_select'; choices: ReportChoice[] }
  | { kind: 'text'; multiline: boolean }
);

export interface ReportSectionSpec {
  key: string;
  title: string;
  description: string;
  report_types: ReportTypeKey[];
  default_enabled: boolean;
  options: ReportOptionSpec[];
}

export interface ReportSectionConfig {
  key: string;
  enabled: boolean;
  options?: Record<string, unknown>;
}

export interface ReportDocumentSettings {
  doc_id: string;
  title: string;
  issue: string;
  revision: string;
  classification: string;
  watermark: string | null;
  page_size: 'a4' | 'letter';
  pdf_a: boolean;
  signatories: { role: string; name: string }[];
  change_record: { issue: string; date: string; changes: string; author: string }[];
  documents: { ref: string; id: string; title: string; issue: string }[];
}

/** A template: the order of `sections` is the order in the document. */
export interface ReportDefinition {
  report_type: ReportTypeKey;
  document: ReportDocumentSettings;
  sections: ReportSectionConfig[];
}

export interface ReportTypeInfo {
  key: ReportTypeKey;
  title: string;
  description: string;
  formats: ReportFormat[];
  sections: ReportSectionSpec[];
  /** The built-in template, filled for this project and the caller. */
  default_definition: ReportDefinition;
}

export interface ReportTemplate {
  id: number;
  project_id: number;
  name: string;
  report_type: ReportTypeKey;
  visibility: 'private' | 'shared';
  owner_id: number;
  owner_name: string;
  definition: ReportDefinition;
  /** Whether the caller may change or delete it. */
  can_edit: boolean;
  created_at: string;
  updated_at: string;
}

export type ReportTemplateBody = {
  name?: string;
  visibility?: 'private' | 'shared';
  definition?: ReportDefinition;
};

export async function listReportTypes(projectId: number): Promise<ReportTypeInfo[]> {
  return fetchJson(`/api/projects/${projectId}/reports/types`);
}

export async function listReportTemplates(projectId: number): Promise<ReportTemplate[]> {
  return fetchJson(`/api/projects/${projectId}/report_templates`);
}

export async function createReportTemplate(
  projectId: number,
  body: ReportTemplateBody,
  csrfToken: string,
): Promise<ReportTemplate> {
  return fetchJson(`/api/projects/${projectId}/report_templates`, {
    method: 'POST',
    headers: { ...JSON_HEADERS, 'X-CSRF-Token': csrfToken },
    body: JSON.stringify(body),
  });
}

export async function updateReportTemplate(
  projectId: number,
  templateId: number,
  body: ReportTemplateBody,
  csrfToken: string,
): Promise<ReportTemplate> {
  return fetchJson(`/api/projects/${projectId}/report_templates/${templateId}`, {
    method: 'PATCH',
    headers: { ...JSON_HEADERS, 'X-CSRF-Token': csrfToken },
    body: JSON.stringify(body),
  });
}

export async function deleteReportTemplate(
  projectId: number,
  templateId: number,
  csrfToken: string,
): Promise<void> {
  await fetchJson(`/api/projects/${projectId}/report_templates/${templateId}`, {
    method: 'DELETE',
    headers: { 'X-CSRF-Token': csrfToken },
  });
}

export type ReportSource = { template_id?: number; definition?: ReportDefinition };

/** Render a report document; resolves with the file and its server name. */
export async function fetchReport(
  projectId: number,
  type: ReportTypeKey,
  format: ReportFormat,
  source: ReportSource,
  csrfToken: string,
): Promise<{ blob: Blob; filename: string | null }> {
  return fetchDownload(`/api/projects/${projectId}/reports/${type}.${format}`, {
    method: 'POST',
    headers: { ...JSON_HEADERS, 'X-CSRF-Token': csrfToken },
    body: JSON.stringify(source),
  });
}

/** Render a report document and save it. */
export async function downloadReport(
  projectId: number,
  type: ReportTypeKey,
  format: ReportFormat,
  source: ReportSource,
  csrfToken: string,
): Promise<void> {
  const { blob, filename } = await fetchReport(projectId, type, format, source, csrfToken);
  triggerDownload(blob, filename ?? `${type}-report.${format}`);
}
