import type {
  EntityActivityItem,
  Requirement,
  RequirementCommentItem,
  RequirementCreateBody,
  RequirementDetailPayload,
  RequirementPatchBody,
  RequirementVersion,
  RequirementVersionLink,
} from './types';
import { fetchJson, JSON_HEADERS } from './transport';

export type RequirementListQuery = {
  approval_state?: string;
  has_tests?: boolean;
  status_id?: number;
  category_id?: number;
  q?: string;
  sort_column?: string;
  sort_dir?: 'asc' | 'desc' | string;
  view_id?: number;
};

export async function listRequirements(
  projectId: number,
  query: RequirementListQuery = {},
): Promise<Requirement[]> {
  const params = new URLSearchParams();
  if (query.approval_state) params.set('approval_state', query.approval_state);
  if (query.has_tests != null) params.set('has_tests', String(query.has_tests));
  if (query.status_id != null) params.set('status_id', String(query.status_id));
  if (query.category_id != null) params.set('category_id', String(query.category_id));
  if (query.q) params.set('q', query.q);
  if (query.sort_column) params.set('sort_column', query.sort_column);
  if (query.sort_dir) params.set('sort_dir', query.sort_dir);
  if (query.view_id != null) params.set('view_id', String(query.view_id));
  const qs = params.toString();
  return fetchJson<Requirement[]>(
    `/api/projects/${projectId}/requirements${qs ? `?${qs}` : ''}`,
  );
}

export async function createRequirementByProject(
  projectId: number,
  body: RequirementCreateBody,
  csrfToken: string,
): Promise<{ id: number }> {
  const res = await fetchJson<{ status?: string; id: number }>(
    `/api/projects/${projectId}/requirements`,
    {
      method: 'POST',
      headers: {
        ...JSON_HEADERS,
        'X-CSRF-Token': csrfToken,
      },
      body: JSON.stringify(body),
    },
  );
  return { id: res.id };
}

export async function getRequirementByProject(
  projectId: number,
  requirementId: number,
): Promise<RequirementDetailPayload> {
  return fetchJson<RequirementDetailPayload>(
    `/api/projects/${projectId}/requirements/${requirementId}`,
  );
}

export async function patchRequirementByProject(
  projectId: number,
  requirementId: number,
  patch: RequirementPatchBody,
  csrfToken: string,
): Promise<void> {
  const body = Object.fromEntries(
    Object.entries(patch).filter(([, v]) => v !== undefined),
  ) as Record<string, unknown>;
  if (Object.keys(body).length === 0) {
    throw new Error('No changes to save');
  }
  await fetchJson(`/api/projects/${projectId}/requirements/${requirementId}`, {
    method: 'PATCH',
    headers: {
      ...JSON_HEADERS,
      'X-CSRF-Token': csrfToken,
    },
    body: JSON.stringify(body),
  });
}

export async function deleteRequirementGlobally(
  requirementId: number,
  csrfToken: string,
): Promise<void> {
  await fetchJson(`/api/requirements/${requirementId}`, {
    method: 'DELETE',
    headers: {
      'X-CSRF-Token': csrfToken,
    },
  });
}

export async function listRequirementVersionsByProject(
  projectId: number,
  requirementId: number,
): Promise<RequirementVersion[]> {
  return fetchJson<RequirementVersion[]>(
    `/api/projects/${projectId}/requirements/${requirementId}/versions`,
  );
}

export async function listRequirementActivityByProject(
  projectId: number,
  requirementId: number,
): Promise<EntityActivityItem[]> {
  return fetchJson(`/api/projects/${projectId}/requirements/${requirementId}/activity`);
}

export async function listRequirementComments(
  requirementId: number,
  versionId?: number,
): Promise<RequirementCommentItem[]> {
  const q =
    versionId != null && Number.isFinite(versionId)
      ? `?version_id=${versionId}`
      : '';
  return fetchJson<RequirementCommentItem[]>(
    `/api/requirements/${requirementId}/comments${q}`,
  );
}

export async function createRequirementComment(
  requirementId: number,
  payload: { body: string; requirement_version_id?: number | null },
  csrfToken: string,
): Promise<RequirementCommentItem> {
  return fetchJson<RequirementCommentItem>(`/api/requirements/${requirementId}/comments`, {
    method: 'POST',
    headers: {
      ...JSON_HEADERS,
      'X-CSRF-Token': csrfToken,
    },
    body: JSON.stringify(payload),
  });
}

export async function listRequirementVersionLinks(
  projectId: number,
  query?: { source_version_id?: number; target_version_id?: number; link_type?: string },
): Promise<RequirementVersionLink[]> {
  const q = new URLSearchParams();
  if (query?.source_version_id != null) {
    q.set('source_version_id', String(query.source_version_id));
  }
  if (query?.target_version_id != null) {
    q.set('target_version_id', String(query.target_version_id));
  }
  if (query?.link_type) q.set('link_type', query.link_type);
  const qs = q.toString();
  return fetchJson<RequirementVersionLink[]>(
    `/api/projects/${projectId}/requirement-version-links${qs ? `?${qs}` : ''}`,
  );
}

export type CreateRequirementVersionLinkBody = {
  source_version_id: number;
  target_version_id: number;
  link_type: string;
  rationale?: string | null;
};

export async function createRequirementVersionLink(
  projectId: number,
  body: CreateRequirementVersionLinkBody,
  csrfToken: string,
): Promise<RequirementVersionLink> {
  return fetchJson<RequirementVersionLink>(
    `/api/projects/${projectId}/requirement-version-links`,
    {
      method: 'POST',
      headers: {
        ...JSON_HEADERS,
        'X-CSRF-Token': csrfToken,
      },
      body: JSON.stringify(body),
    },
  );
}

export async function deleteRequirementVersionLink(
  projectId: number,
  linkId: number,
  csrfToken: string,
): Promise<void> {
  await fetchJson(`/api/projects/${projectId}/requirement-version-links/${linkId}`, {
    method: 'DELETE',
    headers: { 'X-CSRF-Token': csrfToken },
  });
}

export async function listRequirementVersionLinkTypes(projectId: number): Promise<string[]> {
  const r = await fetchJson<{ link_types: string[] }>(
    `/api/projects/${projectId}/requirement-version-links/link-types`,
  );
  return r.link_types ?? [];
}
