import { fetchJson } from './transport';

export interface ExcelColumnPreview {
  index: number;
  name: string;
  sample_value: string;
}

export interface ExcelImportPreview {
  import_type: 'requirements' | 'tests' | string;
  columns: ExcelColumnPreview[];
  sample_rows: string[][];
  row_count: number;
  available_fields: {
    requirements: string[];
    tests: string[];
    matrix?: string[];
  };
  unique_values: Record<string, string[]>;
}

export interface ExcelColumnMapping {
  excel_column: string;
  target_field: string;
}

export interface ExcelValueMapping {
  target_field: string;
  source_value: string;
  target_id: number;
}

export interface ExcelImportResult {
  success: boolean;
  message: string;
  imported_count: number;
  errors: string[];
  imported_requirement_ids: number[];
}

export async function previewExcelImport(
  projectId: number,
  file: File,
  csrfToken: string,
): Promise<ExcelImportPreview> {
  const body = new FormData();
  body.append('file', file);
  return fetchJson<ExcelImportPreview>(`/api/projects/${projectId}/imports/excel/preview`, {
    method: 'POST',
    headers: { 'X-CSRF-Token': csrfToken },
    body,
  });
}

export interface ReqifImportResult {
  success: boolean;
  message: string;
  imported_count: number;
  created_link_count: number;
  errors: string[];
  warnings: string[];
  imported_requirement_ids: number[];
}

export async function commitReqifImport(
  projectId: number,
  file: File,
  csrfToken: string,
): Promise<ReqifImportResult> {
  const body = new FormData();
  body.append('file', file);
  return fetchJson<ReqifImportResult>(`/api/projects/${projectId}/imports/reqif`, {
    method: 'POST',
    headers: { 'X-CSRF-Token': csrfToken },
    body,
  });
}

export async function commitExcelImport(
  projectId: number,
  file: File,
  importType: 'requirements' | 'tests' | 'matrix',
  columnMappings: ExcelColumnMapping[],
  valueMappings: ExcelValueMapping[],
  csrfToken: string,
): Promise<ExcelImportResult> {
  const body = new FormData();
  body.append('file', file);
  body.append('import_type', importType);
  body.append('column_mappings', JSON.stringify(columnMappings));
  body.append('value_mappings', JSON.stringify(valueMappings));
  return fetchJson<ExcelImportResult>(`/api/projects/${projectId}/imports/excel`, {
    method: 'POST',
    headers: { 'X-CSRF-Token': csrfToken },
    body,
  });
}

export interface ProjectBundleImportResult {
  project_id: number;
  slug: string;
  project_base_path: string;
  imported_counts: Record<string, number>;
  warnings: string[];
  errors: string[];
}

export async function importProjectBundle(
  file: File,
  csrfToken: string,
  groupId?: number | null,
): Promise<ProjectBundleImportResult> {
  const body = new FormData();
  body.append('file', file);
  if (groupId != null) {
    body.append('group_id', String(groupId));
  }
  return fetchJson<ProjectBundleImportResult>('/api/projects/imports/bundle', {
    method: 'POST',
    headers: { 'X-CSRF-Token': csrfToken },
    body,
  });
}
