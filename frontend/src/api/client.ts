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
  Requirement,
  RequirementCommentItem,
  RequirementCreateBody,
  RequirementDetailPayload,
  RequirementPatchBody,
  RequirementStatus,
  RequirementVersion,
  User,
  Verification,
  VerificationMethod,
  VerificationStatus,
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
  verificationId: number,
  field: string,
  value: string,
  csrfToken: string,
): Promise<void> {
  await fetchJson(`/api/verifications/${verificationId}/field`, {
    method: 'POST',
    headers: {
      ...JSON_HEADERS,
      'X-CSRF-Token': csrfToken,
    },
    body: JSON.stringify({ field, value }),
  });
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
