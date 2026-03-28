import type {
  Applicability,
  Baseline,
  BaselineTraceabilityRow,
  BaselineVerificationSnapshot,
  Category,
  CoverageReport,
  CustomFieldDefinition,
  DashboardPayload,
  DashboardPayloadWire,
  DashboardProject,
  EffectivePermissions,
  MatrixLink,
  NewVerificationBody,
  ProjectMember,
  ProjectReviewersResponse,
  Requirement,
  RequirementCommentItem,
  RequirementCreateBody,
  RequirementDetailPayload,
  CustomFieldWriteBody,
  RequirementPatchBody,
  RequirementVersionLink,
  RequirementStatus,
  RequirementStatusWriteBody,
  RequirementVersion,
  User,
  Verification,
  TaggedMetadataBody,
  VerificationMatrixPayload,
  VerificationMatrixPutBody,
  VerificationMethod,
  VerificationMethodWriteBody,
  VerificationStatus,
  VerificationStatusWriteBody,
} from './types';

function normalizeDashboard(wire: DashboardPayloadWire): DashboardPayload {
  const projects: DashboardProject[] = wire.projects.map((p) => ({
    ...p,
    id: p.project_id,
    slug: p.project_slug,
  }));
  return {
    ...wire,
    projects,
  };
}

const JSON_HEADERS = { 'Content-Type': 'application/json' };

async function parseJson<T>(res: Response): Promise<T> {
  const text = await res.text();
  if (!res.ok) {
    let msg = res.statusText;
    try {
      const j = JSON.parse(text) as { message?: string; error?: string };
      msg = (j.message ?? j.error ?? text) || msg;
    } catch {
      msg = text || msg;
    }
    throw new Error(msg);
  }
  if (!text) return undefined as T;
  return JSON.parse(text) as T;
}

export async function fetchJson<T>(
  path: string,
  init: RequestInit = {},
): Promise<T> {
  const res = await fetch(path, {
    credentials: 'same-origin',
    ...init,
    headers: {
      ...init.headers,
    },
  });
  return parseJson<T>(res);
}

export async function getCsrfToken(): Promise<string> {
  const data = await fetchJson<{ csrf_token: string }>('/api/auth/csrf');
  return data.csrf_token;
}

export async function loginJson(
  username: string,
  password: string,
  csrfToken: string,
): Promise<void> {
  await fetchJson<{ status: string }>('/api/auth/login', {
    method: 'POST',
    headers: {
      ...JSON_HEADERS,
      'X-CSRF-Token': csrfToken,
    },
    body: JSON.stringify({ username, password }),
  });
}

export async function logoutJson(csrfToken: string): Promise<void> {
  await fetchJson<{ status: string }>('/api/auth/logout', {
    method: 'POST',
    headers: {
      ...JSON_HEADERS,
      'X-CSRF-Token': csrfToken,
    },
    body: '{}',
  });
}

export async function getDashboard(): Promise<DashboardPayload> {
  const wire = await fetchJson<DashboardPayloadWire>('/api/dashboard');
  return normalizeDashboard(wire);
}

export async function listRequirementStatuses(): Promise<RequirementStatus[]> {
  return fetchJson<RequirementStatus[]>('/api/status');
}

export async function listRequirements(projectId: number): Promise<Requirement[]> {
  return fetchJson<Requirement[]>(`/api/projects/${projectId}/requirements`);
}

export async function listMatrix(projectId: number): Promise<MatrixLink[]> {
  return fetchJson<MatrixLink[]>(`/api/projects/${projectId}/matrix`);
}

export async function getVerificationMatrix(
  projectId: number,
  verificationId: number,
): Promise<VerificationMatrixPayload> {
  return fetchJson<VerificationMatrixPayload>(
    `/api/projects/${projectId}/verifications/${verificationId}/matrix`,
  );
}

/** Replace traceability links for this verification (full list; empty array unlinks all). */
export async function putVerificationMatrix(
  projectId: number,
  verificationId: number,
  body: VerificationMatrixPutBody,
  csrfToken: string,
): Promise<VerificationMatrixPayload & { status: string }> {
  return fetchJson<VerificationMatrixPayload & { status: string }>(
    `/api/projects/${projectId}/verifications/${verificationId}/matrix`,
    {
      method: 'PUT',
      headers: {
        ...JSON_HEADERS,
        'X-CSRF-Token': csrfToken,
      },
      body: JSON.stringify(body),
    },
  );
}

/** All verifications (filter by project_id client-side). */
export async function listVerifications(): Promise<Verification[]> {
  return fetchJson<Verification[]>('/api/verifications');
}

export async function listVerificationStatuses(): Promise<VerificationStatus[]> {
  return fetchJson<VerificationStatus[]>('/api/verification-status');
}

export async function listVerificationMethodsByProject(
  projectId: number,
): Promise<VerificationMethod[]> {
  return fetchJson<VerificationMethod[]>(
    `/api/projects/${projectId}/verification-methods`,
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

export async function createVerification(
  body: NewVerificationBody,
  csrfToken: string,
): Promise<{ id: number }> {
  const res = await fetchJson<{ status?: string; id: number }>('/api/verifications', {
    method: 'POST',
    headers: {
      ...JSON_HEADERS,
      'X-CSRF-Token': csrfToken,
    },
    body: JSON.stringify(body),
  });
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

export async function listRequirementVersionsByProject(
  projectId: number,
  requirementId: number,
): Promise<RequirementVersion[]> {
  return fetchJson<RequirementVersion[]>(
    `/api/projects/${projectId}/requirements/${requirementId}/versions`,
  );
}

export async function listCategories(): Promise<Category[]> {
  return fetchJson<Category[]>('/api/categories');
}

export async function listApplicability(): Promise<Applicability[]> {
  return fetchJson<Applicability[]>('/api/applicability');
}

export async function listProjectMembers(projectId: number): Promise<ProjectMember[]> {
  return fetchJson<ProjectMember[]>(`/api/projects/${projectId}/members`);
}

export async function getProjectReviewers(
  projectId: number,
): Promise<ProjectReviewersResponse> {
  return fetchJson<ProjectReviewersResponse>(
    `/api/projects/${projectId}/reviewers`,
  );
}

export async function putProjectReviewers(
  projectId: number,
  userIds: number[],
  csrfToken: string,
): Promise<ProjectReviewersResponse> {
  return fetchJson<ProjectReviewersResponse>(
    `/api/projects/${projectId}/reviewers`,
    {
      method: 'PUT',
      headers: {
        ...JSON_HEADERS,
        'X-CSRF-Token': csrfToken,
      },
      body: JSON.stringify({ user_ids: userIds }),
    },
  );
}

export async function getCoverageReport(projectId: number): Promise<CoverageReport> {
  return fetchJson<CoverageReport>(`/api/projects/${projectId}/coverage_report`);
}

export async function getMyPermissions(projectId: number): Promise<EffectivePermissions> {
  return fetchJson<EffectivePermissions>(
    `/api/projects/${projectId}/me/permissions`,
  );
}

export async function listCustomFieldsByProject(
  projectId: number,
): Promise<CustomFieldDefinition[]> {
  return fetchJson<CustomFieldDefinition[]>(
    `/api/projects/${projectId}/custom_fields`,
  );
}

export async function getVerification(verificationId: number): Promise<Verification> {
  return fetchJson<Verification>(`/api/verifications/${verificationId}`);
}

export async function updateVerificationField(
  projectId: number,
  verificationId: number,
  field: string,
  value: string,
  csrfToken: string,
): Promise<void> {
  await fetchJson(
    `/api/projects/${projectId}/verifications/${verificationId}/field`,
    {
      method: 'POST',
      headers: {
        ...JSON_HEADERS,
        'X-CSRF-Token': csrfToken,
      },
      body: JSON.stringify({ field, value }),
    },
  );
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

export async function deleteVerificationGlobally(
  verificationId: number,
  csrfToken: string,
): Promise<void> {
  await fetchJson(`/api/verifications/${verificationId}`, {
    method: 'DELETE',
    headers: {
      'X-CSRF-Token': csrfToken,
    },
  });
}

export async function clearTraceabilitySuspect(
  reqId: number,
  verificationId: number,
  csrfToken: string,
): Promise<void> {
  await fetchJson('/api/traceability/clear_suspect', {
    method: 'POST',
    headers: {
      ...JSON_HEADERS,
      'X-CSRF-Token': csrfToken,
    },
    body: JSON.stringify({
      req_id: reqId,
      verification_id: verificationId,
    }),
  });
}

export async function listBaselines(projectId: number): Promise<Baseline[]> {
  return fetchJson<Baseline[]>(`/api/projects/${projectId}/baselines`);
}

export async function getBaseline(
  projectId: number,
  baselineId: number,
): Promise<Baseline> {
  return fetchJson<Baseline>(`/api/projects/${projectId}/baselines/${baselineId}`);
}

export async function createBaseline(
  projectId: number,
  name: string,
  description: string | null | undefined,
  csrfToken: string,
): Promise<Baseline> {
  return fetchJson<Baseline>(`/api/projects/${projectId}/baselines`, {
    method: 'POST',
    headers: {
      ...JSON_HEADERS,
      'X-CSRF-Token': csrfToken,
    },
    body: JSON.stringify({
      name,
      description: description?.trim() ? description.trim() : null,
    }),
  });
}

export async function getBaselineRequirements(
  projectId: number,
  baselineId: number,
): Promise<Requirement[]> {
  return fetchJson<Requirement[]>(
    `/api/projects/${projectId}/baselines/${baselineId}/requirements`,
  );
}

export async function getBaselineTraceability(
  projectId: number,
  baselineId: number,
): Promise<BaselineTraceabilityRow[]> {
  return fetchJson<BaselineTraceabilityRow[]>(
    `/api/projects/${projectId}/baselines/${baselineId}/traceability`,
  );
}

export async function getBaselineVerifications(
  projectId: number,
  baselineId: number,
): Promise<BaselineVerificationSnapshot[]> {
  return fetchJson<BaselineVerificationSnapshot[]>(
    `/api/projects/${projectId}/baselines/${baselineId}/verifications`,
  );
}

export async function setProjectMemberRole(
  projectId: number,
  userId: number,
  role: number,
  csrfToken: string,
): Promise<ProjectMember> {
  return fetchJson<ProjectMember>(`/api/projects/${projectId}/members/${userId}`, {
    method: 'PUT',
    headers: {
      ...JSON_HEADERS,
      'X-CSRF-Token': csrfToken,
    },
    body: JSON.stringify({ role }),
  });
}

export async function removeProjectMember(
  projectId: number,
  userId: number,
  csrfToken: string,
): Promise<void> {
  await fetchJson(`/api/projects/${projectId}/members/${userId}`, {
    method: 'DELETE',
    headers: {
      'X-CSRF-Token': csrfToken,
    },
  });
}

/** Admin-only; returns null if forbidden. */
export async function listUsersOptional(): Promise<User[] | null> {
  try {
    return await fetchJson<User[]>('/api/users');
  } catch {
    return null;
  }
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
  return fetchJson<RequirementVersionLink>(`/api/projects/${projectId}/requirement-version-links`, {
    method: 'POST',
    headers: {
      ...JSON_HEADERS,
      'X-CSRF-Token': csrfToken,
    },
    body: JSON.stringify(body),
  });
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

/* ——— Project catalog (categories, applicability, statuses, custom fields, methods) ——— */

export async function createCategory(
  body: TaggedMetadataBody,
  csrfToken: string,
): Promise<{ id: number }> {
  const r = await fetchJson<{ id: number }>('/api/categories', {
    method: 'POST',
    headers: { ...JSON_HEADERS, 'X-CSRF-Token': csrfToken },
    body: JSON.stringify({ ...body, id: body.id ?? null }),
  });
  return { id: r.id };
}

export async function updateCategory(
  id: number,
  body: TaggedMetadataBody,
  csrfToken: string,
): Promise<void> {
  await fetchJson(`/api/categories/${id}`, {
    method: 'PUT',
    headers: { ...JSON_HEADERS, 'X-CSRF-Token': csrfToken },
    body: JSON.stringify({ ...body, id }),
  });
}

export async function deleteCategory(id: number, csrfToken: string): Promise<void> {
  await fetchJson(`/api/categories/${id}`, {
    method: 'DELETE',
    headers: { 'X-CSRF-Token': csrfToken },
  });
}

export async function createApplicability(
  body: TaggedMetadataBody,
  csrfToken: string,
): Promise<{ id: number }> {
  const res = await fetch('/api/applicability', {
    method: 'POST',
    credentials: 'same-origin',
    headers: { ...JSON_HEADERS, 'X-CSRF-Token': csrfToken },
    body: JSON.stringify({ ...body, id: body.id ?? null }),
  });
  if (!res.ok) {
    const t = await res.text();
    throw new Error(t || res.statusText);
  }
  return (await res.json()) as { id: number };
}

export async function updateApplicability(
  id: number,
  body: TaggedMetadataBody,
  csrfToken: string,
): Promise<void> {
  await fetchJson(`/api/applicability/${id}`, {
    method: 'PUT',
    headers: { ...JSON_HEADERS, 'X-CSRF-Token': csrfToken },
    body: JSON.stringify({ ...body, id }),
  });
}

export async function deleteApplicability(id: number, csrfToken: string): Promise<void> {
  await fetchJson(`/api/applicability/${id}`, {
    method: 'DELETE',
    headers: { 'X-CSRF-Token': csrfToken },
  });
}

export async function createRequirementStatus(
  body: RequirementStatusWriteBody,
  csrfToken: string,
): Promise<{ id: number }> {
  const res = await fetch('/api/status', {
    method: 'POST',
    credentials: 'same-origin',
    headers: { ...JSON_HEADERS, 'X-CSRF-Token': csrfToken },
    body: JSON.stringify({
      title: body.title,
      description: body.description,
      tag: body.tag,
      project_id: body.project_id,
      tag_color: body.tag_color ?? null,
    }),
  });
  if (!res.ok) {
    const t = await res.text();
    throw new Error(t || res.statusText);
  }
  const j = (await res.json()) as { id: number };
  return { id: j.id };
}

export async function updateRequirementStatus(
  id: number,
  body: RequirementStatusWriteBody,
  csrfToken: string,
): Promise<void> {
  await fetchJson(`/api/status/${id}`, {
    method: 'PUT',
    headers: { ...JSON_HEADERS, 'X-CSRF-Token': csrfToken },
    body: JSON.stringify({
      id,
      title: body.title,
      description: body.description,
      tag: body.tag,
      project_id: body.project_id,
      is_system: body.is_system ?? false,
      tag_color: body.tag_color ?? null,
    }),
  });
}

export async function deleteRequirementStatus(id: number, csrfToken: string): Promise<void> {
  await fetchJson(`/api/status/${id}`, {
    method: 'DELETE',
    headers: { 'X-CSRF-Token': csrfToken },
  });
}

export async function createVerificationStatus(
  body: VerificationStatusWriteBody,
  csrfToken: string,
): Promise<{ id: number }> {
  const res = await fetch('/api/verification-status', {
    method: 'POST',
    credentials: 'same-origin',
    headers: { ...JSON_HEADERS, 'X-CSRF-Token': csrfToken },
    body: JSON.stringify({
      title: body.title,
      description: body.description,
      tag: body.tag,
      project_id: body.project_id,
      tag_color: body.tag_color ?? null,
    }),
  });
  if (!res.ok) {
    const t = await res.text();
    throw new Error(t || res.statusText);
  }
  const j = (await res.json()) as { id: number };
  return { id: j.id };
}

export async function updateVerificationStatus(
  id: number,
  body: VerificationStatusWriteBody,
  csrfToken: string,
): Promise<void> {
  await fetchJson(`/api/verification-status/${id}`, {
    method: 'PUT',
    headers: { ...JSON_HEADERS, 'X-CSRF-Token': csrfToken },
    body: JSON.stringify({
      id,
      title: body.title,
      description: body.description,
      tag: body.tag,
      project_id: body.project_id,
      is_system: body.is_system ?? false,
      tag_color: body.tag_color ?? null,
    }),
  });
}

export async function deleteVerificationStatus(id: number, csrfToken: string): Promise<void> {
  await fetchJson(`/api/verification-status/${id}`, {
    method: 'DELETE',
    headers: { 'X-CSRF-Token': csrfToken },
  });
}

export async function createCustomField(
  projectId: number,
  body: CustomFieldWriteBody,
  csrfToken: string,
): Promise<{ id: number }> {
  const r = await fetchJson<{ id: number }>(
    `/api/projects/${projectId}/custom_fields`,
    {
      method: 'POST',
      headers: { ...JSON_HEADERS, 'X-CSRF-Token': csrfToken },
      body: JSON.stringify(body),
    },
  );
  return { id: r.id };
}

export async function updateCustomField(
  projectId: number,
  fieldId: number,
  body: CustomFieldWriteBody,
  csrfToken: string,
): Promise<void> {
  await fetchJson(`/api/projects/${projectId}/custom_fields/${fieldId}`, {
    method: 'PUT',
    headers: { ...JSON_HEADERS, 'X-CSRF-Token': csrfToken },
    body: JSON.stringify(body),
  });
}

export async function deleteCustomField(
  projectId: number,
  fieldId: number,
  csrfToken: string,
): Promise<void> {
  await fetchJson(`/api/projects/${projectId}/custom_fields/${fieldId}`, {
    method: 'DELETE',
    headers: { ...JSON_HEADERS, 'X-CSRF-Token': csrfToken },
  });
}

export async function createVerificationMethod(
  projectId: number,
  body: VerificationMethodWriteBody,
  csrfToken: string,
): Promise<{ id: number }> {
  const r = await fetchJson<{ id: number }>(
    `/api/projects/${projectId}/verification-methods`,
    {
      method: 'POST',
      headers: { ...JSON_HEADERS, 'X-CSRF-Token': csrfToken },
      body: JSON.stringify({ ...body, id: body.id ?? null, project_id: projectId }),
    },
  );
  return { id: r.id };
}

export async function updateVerificationMethod(
  projectId: number,
  methodId: number,
  body: VerificationMethodWriteBody,
  csrfToken: string,
): Promise<void> {
  await fetchJson(`/api/projects/${projectId}/verification-methods/${methodId}`, {
    method: 'PUT',
    headers: { ...JSON_HEADERS, 'X-CSRF-Token': csrfToken },
    body: JSON.stringify({ ...body, id: methodId, project_id: projectId }),
  });
}

export async function deleteVerificationMethod(
  projectId: number,
  methodId: number,
  csrfToken: string,
): Promise<void> {
  await fetchJson(`/api/projects/${projectId}/verification-methods/${methodId}`, {
    method: 'DELETE',
    headers: { 'X-CSRF-Token': csrfToken },
  });
}
