import type {
  BaselineUseCase,
  UseCaseComment,
  UseCaseDetail,
  UseCaseDiff,
  UseCaseDocument,
  UseCaseDraft,
  UseCaseDraftChange,
  UseCaseLink,
  UseCaseOperation,
  UseCaseRef,
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
  has_suspect?: string;
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
): Promise<{ version: UseCaseVersion; removed_links: UseCaseLink[] }> {
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

export async function listUseCaseLinks(projectId: number, id: number): Promise<UseCaseLink[]> {
  return fetchJson(`${base(projectId)}/${id}/links`);
}

export async function createUseCaseLink(
  projectId: number,
  id: number,
  body: {
    element_id?: string | null;
    requirement_id?: number;
    verification_id?: number;
    rationale?: string | null;
  },
  csrfToken: string,
): Promise<UseCaseLink> {
  return fetchJson(`${base(projectId)}/${id}/links`, write('POST', body, csrfToken));
}

export async function deleteUseCaseLink(
  projectId: number,
  id: number,
  linkId: number,
  csrfToken: string,
): Promise<void> {
  await fetchJson(`${base(projectId)}/${id}/links/${linkId}`, write('DELETE', undefined, csrfToken));
}

export async function clearUseCaseLinkSuspect(
  projectId: number,
  id: number,
  linkId: number,
  csrfToken: string,
): Promise<{ status: string; cleared: boolean }> {
  return fetchJson(
    `${base(projectId)}/${id}/links/${linkId}/clear_suspect`,
    write('POST', undefined, csrfToken),
  );
}

/** Use cases that link to a requirement ("Used in use cases"). */
export async function listRequirementUseCases(
  projectId: number,
  requirementId: number,
): Promise<UseCaseRef[]> {
  return fetchJson(`/api/projects/${projectId}/requirements/${requirementId}/use-cases`);
}

export async function listVerificationUseCases(
  projectId: number,
  verificationId: number,
): Promise<UseCaseRef[]> {
  return fetchJson(`/api/projects/${projectId}/verifications/${verificationId}/use-cases`);
}

/** The draft against the version it started from. */
export async function getUseCaseDraftDiff(
  projectId: number,
  id: number,
  includeUnchanged = false,
): Promise<UseCaseDiff> {
  return fetchJson(`${base(projectId)}/${id}/draft/diff${includeUnchanged ? '?include_unchanged=true' : ''}`);
}

/** Version `from` (before) against version `to` (after). */
export async function getUseCaseVersionDiff(
  projectId: number,
  id: number,
  from: number,
  to: number,
  includeUnchanged = false,
): Promise<UseCaseDiff> {
  return fetchJson(
    `${base(projectId)}/${id}/versions/${from}/diff/${to}${includeUnchanged ? '?include_unchanged=true' : ''}`,
  );
}

export async function listUseCaseComments(projectId: number, id: number): Promise<UseCaseComment[]> {
  return fetchJson(`${base(projectId)}/${id}/comments`);
}

export async function createUseCaseComment(
  projectId: number,
  id: number,
  body: { body: string; use_case_version_id?: number | null; element_id?: string | null },
  csrfToken: string,
): Promise<UseCaseComment> {
  return fetchJson(`${base(projectId)}/${id}/comments`, write('POST', body, csrfToken));
}

/** The use case versions a baseline recorded. */
export async function listBaselineUseCases(
  projectId: number,
  baselineId: number,
): Promise<BaselineUseCase[]> {
  return fetchJson(`/api/projects/${projectId}/baselines/${baselineId}/use-cases`);
}
