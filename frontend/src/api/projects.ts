import type {
  CoverageReport,
  EffectivePermissions,
  Project,
  ProjectFromPath,
  ProjectMember,
  ProjectReviewersResponse,
  ProjectUpdateBody,
} from './types';
import { fetchJson, JSON_HEADERS } from './transport';

export async function createProject(
  body: { name: string; description?: string | null; group_id?: number | null },
  csrfToken: string,
): Promise<{
  id: number;
  name: string;
  slug: string;
  group_id: number | null;
  project_base_path: string;
}> {
  return fetchJson('/api/projects', {
    method: 'POST',
    headers: { ...JSON_HEADERS, 'X-CSRF-Token': csrfToken },
    body: JSON.stringify(body),
  });
}

/** Edit project properties (project Admin or instance admin). Returns the updated project. */
export async function updateProject(
  projectId: number,
  body: ProjectUpdateBody,
  csrfToken: string,
): Promise<Project & { project_base_path: string }> {
  return fetchJson(`/api/projects/${projectId}`, {
    method: 'PATCH',
    headers: { ...JSON_HEADERS, 'X-CSRF-Token': csrfToken },
    body: JSON.stringify(body),
  });
}

/**
 * Permanently delete a project and all its data (owner or instance admin).
 * `confirmSlug` must equal the project's slug.
 */
export async function deleteProject(
  projectId: number,
  confirmSlug: string,
  csrfToken: string,
): Promise<void> {
  await fetchJson(`/api/projects/${projectId}`, {
    method: 'DELETE',
    headers: { ...JSON_HEADERS, 'X-CSRF-Token': csrfToken },
    body: JSON.stringify({ confirm_slug: confirmSlug }),
  });
}

/**
 * Archive (read-only for everyone) or unarchive a project; owner or instance
 * admin only (issue #381). Returns the updated project.
 */
export async function setProjectArchived(
  projectId: number,
  archived: boolean,
  csrfToken: string,
): Promise<Project> {
  return fetchJson(`/api/projects/${projectId}/${archived ? 'archive' : 'unarchive'}`, {
    method: 'POST',
    headers: { ...JSON_HEADERS, 'X-CSRF-Token': csrfToken },
  });
}

export async function getProjectFromPath(slug: string): Promise<ProjectFromPath> {
  return fetchJson<ProjectFromPath>(`/api/project-from-path/${encodeURIComponent(slug)}`);
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
