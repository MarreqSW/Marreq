import type {
  GroupMemberResponse,
  GroupResponse,
  Project,
} from './types';
import { fetchJson, JSON_HEADERS } from './transport';

export async function listGroups(): Promise<GroupResponse[]> {
  return fetchJson<GroupResponse[]>('/api/groups');
}

export async function getGroup(id: number): Promise<GroupResponse> {
  return fetchJson<GroupResponse>(`/api/groups/${id}`);
}

export async function createGroup(
  body: { name: string; description?: string | null },
  csrfToken: string,
): Promise<GroupResponse> {
  return fetchJson<GroupResponse>('/api/groups', {
    method: 'POST',
    headers: { ...JSON_HEADERS, 'X-CSRF-Token': csrfToken },
    body: JSON.stringify(body),
  });
}

export async function updateGroup(
  id: number,
  body: { name: string; description?: string | null },
  csrfToken: string,
): Promise<GroupResponse> {
  return fetchJson<GroupResponse>(`/api/groups/${id}`, {
    method: 'PATCH',
    headers: { ...JSON_HEADERS, 'X-CSRF-Token': csrfToken },
    body: JSON.stringify(body),
  });
}

export async function deleteGroup(id: number, csrfToken: string): Promise<void> {
  await fetchJson(`/api/groups/${id}`, {
    method: 'DELETE',
    headers: { 'X-CSRF-Token': csrfToken },
  });
}

export async function listGroupProjects(id: number): Promise<Project[]> {
  return fetchJson<Project[]>(`/api/groups/${id}/projects`);
}

export async function listGroupMembers(id: number): Promise<GroupMemberResponse[]> {
  return fetchJson<GroupMemberResponse[]>(`/api/groups/${id}/members`);
}

export async function setGroupMemberRole(
  groupId: number,
  userId: number,
  role: number,
  csrfToken: string,
): Promise<GroupMemberResponse> {
  return fetchJson<GroupMemberResponse>(`/api/groups/${groupId}/members/${userId}`, {
    method: 'PUT',
    headers: { ...JSON_HEADERS, 'X-CSRF-Token': csrfToken },
    body: JSON.stringify({ role }),
  });
}

export async function removeGroupMember(
  groupId: number,
  userId: number,
  csrfToken: string,
): Promise<void> {
  await fetchJson(`/api/groups/${groupId}/members/${userId}`, {
    method: 'DELETE',
    headers: { 'X-CSRF-Token': csrfToken },
  });
}
