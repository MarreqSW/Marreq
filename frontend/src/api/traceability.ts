import type { Dsm } from './types';
import { fetchJson, JSON_HEADERS } from './transport';
import { dsmQueryString, type DsmParams } from '@/utils/dsm';

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

/** Dependency structure matrix for the project (requirement × requirement). */
export async function getDsm(projectId: number, params: DsmParams): Promise<Dsm> {
  return fetchJson<Dsm>(`/api/projects/${projectId}/dsm${dsmQueryString(params)}`);
}
