import type {
  EntityActivityItem,
  MatrixLink,
  NewVerificationBody,
  Verification,
  VerificationMatrixPayload,
  VerificationMatrixPutBody,
  VerificationMethod,
} from './types';
import { fetchJson, JSON_HEADERS } from './transport';

export async function listVerifications(): Promise<Verification[]> {
  return fetchJson<Verification[]>('/api/verifications');
}

export async function listVerificationMethodsByProject(
  projectId: number,
): Promise<VerificationMethod[]> {
  return fetchJson<VerificationMethod[]>(
    `/api/projects/${projectId}/verification-methods`,
  );
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

export async function listVerificationActivityByProject(
  projectId: number,
  verificationId: number,
): Promise<EntityActivityItem[]> {
  return fetchJson(`/api/projects/${projectId}/verifications/${verificationId}/activity`);
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
