import type {
  Compliance,
  RequirementCloseOut,
  VerificationControl,
  VerificationControlBody,
} from './types';
import { fetchJson, JSON_HEADERS } from './transport';

/** Level, stage and evidence of a verification, with suggestions (issue #353). */
export async function getVerificationControl(
  projectId: number,
  verificationId: number,
): Promise<VerificationControl> {
  return fetchJson(`/api/projects/${projectId}/verifications/${verificationId}/control`);
}

/** Replace level, stage and evidence; empty strings clear a field (EditRequirements). */
export async function putVerificationControl(
  projectId: number,
  verificationId: number,
  body: VerificationControlBody,
  csrfToken: string,
): Promise<VerificationControl> {
  return fetchJson(`/api/projects/${projectId}/verifications/${verificationId}/control`, {
    method: 'PUT',
    headers: { ...JSON_HEADERS, 'X-CSRF-Token': csrfToken },
    body: JSON.stringify(body),
  });
}

/** Compliance assessment, linked verifications and derived close-out of a requirement. */
export async function getRequirementCloseOut(
  projectId: number,
  requirementId: number,
): Promise<RequirementCloseOut> {
  return fetchJson(`/api/projects/${projectId}/requirements/${requirementId}/close_out`);
}

/** Close-out of every requirement in the project, keyed by requirement id. */
export async function getProjectCloseOut(
  projectId: number,
): Promise<Record<string, RequirementCloseOut>> {
  return fetchJson(`/api/projects/${projectId}/close_out`);
}

/** Set (C / PC / NC) or clear (`null`) the assessment. Project reviewers only. */
export async function putRequirementCompliance(
  projectId: number,
  requirementId: number,
  compliance: Compliance | null,
  note: string | null,
  csrfToken: string,
): Promise<RequirementCloseOut> {
  return fetchJson(`/api/projects/${projectId}/requirements/${requirementId}/compliance`, {
    method: 'PUT',
    headers: { ...JSON_HEADERS, 'X-CSRF-Token': csrfToken },
    body: JSON.stringify({ compliance, note }),
  });
}
