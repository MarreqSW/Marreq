import type { SavedView, SavedViewPayload } from './types';
import { fetchJson, JSON_HEADERS } from './transport';

export async function listSavedViews(projectId: number): Promise<SavedView[]> {
  return fetchJson<SavedView[]>(`/api/projects/${projectId}/saved_views`);
}

export async function getSavedView(projectId: number, viewId: number): Promise<SavedView> {
  return fetchJson<SavedView>(`/api/projects/${projectId}/saved_views/${viewId}`);
}

export async function createSavedView(
  projectId: number,
  payload: SavedViewPayload,
  csrfToken: string,
): Promise<SavedView> {
  return fetchJson<SavedView>(`/api/projects/${projectId}/saved_views`, {
    method: 'POST',
    headers: {
      ...JSON_HEADERS,
      'X-CSRF-Token': csrfToken,
    },
    body: JSON.stringify(payload),
  });
}

export async function updateSavedView(
  projectId: number,
  viewId: number,
  payload: SavedViewPayload,
  csrfToken: string,
): Promise<SavedView> {
  return fetchJson<SavedView>(`/api/projects/${projectId}/saved_views/${viewId}`, {
    method: 'PATCH',
    headers: {
      ...JSON_HEADERS,
      'X-CSRF-Token': csrfToken,
    },
    body: JSON.stringify(payload),
  });
}

export async function deleteSavedView(
  projectId: number,
  viewId: number,
  csrfToken: string,
): Promise<void> {
  await fetchJson(`/api/projects/${projectId}/saved_views/${viewId}`, {
    method: 'DELETE',
    headers: {
      'X-CSRF-Token': csrfToken,
    },
  });
}
