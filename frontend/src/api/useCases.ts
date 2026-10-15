import type {
  UseCaseDetail,
  UseCaseDocument,
  UseCaseDraft,
  UseCaseDraftChange,
  UseCaseOperation,
  UseCaseRow,
  UseCaseVersion,
  UseCaseVersionSummary,
} from './types';
import { ApiError, fetchJson, JSON_HEADERS } from './transport';

/** Use cases (issue #398): see docs/developer/use-cases-api.md. */
const base = (projectId: number) => `/api/projects/${projectId}/use-cases`;

function write(method: string, body: unknown, csrfToken: string): RequestInit {
  return {
    method,
    headers: { ...JSON_HEADERS, 'X-CSRF-Token': csrfToken },
    body: body === undefined ? undefined : JSON.stringify(body),
  };
}

export interface UseCaseListQuery {
  q?: string;
  approval_state?: string;
  actor?: string;
  sort_column?: string;
  sort_dir?: 'asc' | 'desc';
}

export async function listUseCases(
  projectId: number,
  query: UseCaseListQuery = {},
): Promise<UseCaseRow[]> {
  const params = new URLSearchParams();
  for (const [k, v] of Object.entries(query)) if (v) params.set(k, v);
  const qs = params.toString();
  return fetchJson(`${base(projectId)}${qs ? `?${qs}` : ''}`);
}

export async function getNextUseCaseCode(projectId: number): Promise<string> {
  const r = await fetchJson<{ reference_code: string }>(`${base(projectId)}/next-code`);
  return r.reference_code;
}

export async function createUseCase(
  projectId: number,
  body: { reference_code?: string; title: string; primary_actor?: string },
  csrfToken: string,
): Promise<UseCaseDetail> {
  return fetchJson(base(projectId), write('POST', body, csrfToken));
}

/** The use case with its current version, or `versionId` when given. */
export async function getUseCase(
  projectId: number,
  id: number,
  versionId?: number,
): Promise<UseCaseDetail> {
  const qs = versionId != null ? `?version_id=${versionId}` : '';
  return fetchJson(`${base(projectId)}/${id}${qs}`);
}

export async function deleteUseCase(projectId: number, id: number, csrfToken: string): Promise<void> {
  await fetchJson(`${base(projectId)}/${id}`, write('DELETE', undefined, csrfToken));
}

/** The shared draft, or `null` when there is none. */
export async function getUseCaseDraft(projectId: number, id: number): Promise<UseCaseDraft | null> {
  try {
    return await fetchJson<UseCaseDraft>(`${base(projectId)}/${id}/draft`);
  } catch (e) {
    if (e instanceof ApiError && e.status === 404) return null;
    throw e;
  }
}

/** Apply operations atomically. `baseRevision` is `null` to open a draft from the current version. */
export async function applyUseCaseOperations(
  projectId: number,
  id: number,
  baseRevision: number | null,
  operations: UseCaseOperation[],
  csrfToken: string,
): Promise<UseCaseDraftChange> {
  return fetchJson(
    `${base(projectId)}/${id}/draft/operations`,
    write('POST', { base_revision: baseRevision, operations }, csrfToken),
  );
}

export async function putUseCaseDraft(
  projectId: number,
  id: number,
  baseRevision: number | null,
  document: UseCaseDocument,
  csrfToken: string,
): Promise<UseCaseDraftChange> {
  return fetchJson(
    `${base(projectId)}/${id}/draft`,
    write('PUT', { base_revision: baseRevision, document }, csrfToken),
  );
}

export async function discardUseCaseDraft(
  projectId: number,
  id: number,
  csrfToken: string,
): Promise<void> {
  await fetchJson(`${base(projectId)}/${id}/draft`, write('DELETE', undefined, csrfToken));
}

export async function publishUseCaseDraft(
  projectId: number,
  id: number,
  baseRevision: number,
  changeNote: string | null,
  csrfToken: string,
): Promise<{ version: UseCaseVersion }> {
  return fetchJson(
    `${base(projectId)}/${id}/draft/publish`,
    write('POST', { base_revision: baseRevision, change_note: changeNote }, csrfToken),
  );
}

export async function listUseCaseVersions(
  projectId: number,
  id: number,
): Promise<UseCaseVersionSummary[]> {
  return fetchJson(`${base(projectId)}/${id}/versions`);
}

export async function setUseCaseVersionApproval(
  projectId: number,
  id: number,
  versionId: number,
  state: 'reviewed' | 'approved',
  csrfToken: string,
): Promise<UseCaseVersion> {
  return fetchJson(
    `${base(projectId)}/${id}/versions/${versionId}/approval`,
    write('PUT', { state }, csrfToken),
  );
}
