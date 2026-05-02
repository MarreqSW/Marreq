import type { Notification, NotificationPreference } from './types';
import { fetchJson, JSON_HEADERS } from './transport';

export async function getNotifications(
  unreadOnly = false,
  limit = 50,
): Promise<Notification[]> {
  const params = new URLSearchParams();
  if (unreadOnly) params.set('unread_only', 'true');
  params.set('limit', String(limit));
  return fetchJson<Notification[]>(`/api/notifications?${params}`);
}

export async function getUnreadCount(): Promise<number> {
  const data = await fetchJson<{ count: number }>('/api/notifications/unread-count');
  return data.count;
}

export async function markNotificationRead(id: number, csrfToken: string): Promise<void> {
  await fetchJson(`/api/notifications/${id}/read`, {
    method: 'PATCH',
    headers: { 'X-CSRF-Token': csrfToken },
  });
}

export async function markAllNotificationsRead(csrfToken: string): Promise<void> {
  await fetchJson('/api/notifications/read-all', {
    method: 'POST',
    headers: { 'X-CSRF-Token': csrfToken },
  });
}

export async function getNotificationPreferences(): Promise<NotificationPreference[]> {
  return fetchJson<NotificationPreference[]>('/api/notifications/preferences');
}

export async function setNotificationPreference(
  projectId: number,
  payload: { notify_in_app?: boolean; notify_email?: boolean },
  csrfToken: string,
): Promise<void> {
  await fetchJson(`/api/notifications/preferences/${projectId}`, {
    method: 'PUT',
    headers: { ...JSON_HEADERS, 'X-CSRF-Token': csrfToken },
    body: JSON.stringify(payload),
  });
}

export async function deleteNotificationPreference(
  projectId: number,
  csrfToken: string,
): Promise<void> {
  await fetchJson(`/api/notifications/preferences/${projectId}`, {
    method: 'DELETE',
    headers: { 'X-CSRF-Token': csrfToken },
  });
}
