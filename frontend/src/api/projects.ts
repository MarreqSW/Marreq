import type {
  CoverageReport,
  EffectivePermissions,
  Project,
  ProjectFromPath,
  ProjectMember,
  ProjectReviewersResponse,
} from './types';
import { fetchJson, JSON_HEADERS } from './transport';

export async function createProject(
  body: { name: string; description?: string | null; group_id?: number | null },
  csrfToken: string,
): Promise<{ id: number; name: string; slug: string; group_id: number | null }> {
  return fetchJson('/api/projects', {
    method: 'POST',
    headers: { ...JSON_HEADERS, 'X-CSRF-Token': csrfToken },
    body: JSON.stringify(body),
  });
}

export async function getProjectFromPath(
  namespace: string,
  slug: string,
): Promise<ProjectFromPath> {
  return fetchJson<ProjectFromPath>(`/api/project-from-path/${namespace}/${slug}`);
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

export async function getCoverageReport(projectId: number): Promise<CoverageReport> {
  return fetchJson<CoverageReport>(`/api/projects/${projectId}/coverage_report`);
}

export async function getMyPermissions(projectId: number): Promise<EffectivePermissions> {
  return fetchJson<EffectivePermissions>(
    `/api/projects/${projectId}/me/permissions`,
  );
}

/** Admin-only; returns null if not a member or forbidden. */
export async function listProjectsOptional(): Promise<Project[] | null> {
  try {
    return await fetchJson<Project[]>('/api/projects');
  } catch {
    return null;
  }
}
