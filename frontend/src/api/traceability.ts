import { fetchJson, JSON_HEADERS } from './transport';

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
