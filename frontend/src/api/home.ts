import type { AttentionResponse, SearchHit } from './types';
import { fetchJson } from './transport';

/** What needs the user's attention across their projects (issue #387). */
export async function getAttention(): Promise<AttentionResponse> {
  return fetchJson<AttentionResponse>('/api/home/attention');
}

/** Requirements whose reference code or title contains `q`, across the user's projects. */
export async function searchRequirements(q: string, limit = 8): Promise<SearchHit[]> {
  const params = new URLSearchParams({ q, limit: String(limit) });
  return fetchJson<SearchHit[]>(`/api/search?${params}`);
}
