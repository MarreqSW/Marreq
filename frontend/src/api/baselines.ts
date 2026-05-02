import type {
  Baseline,
  BaselineTraceabilityRow,
  BaselineVerificationSnapshot,
  Requirement,
} from './types';
import { fetchJson, JSON_HEADERS } from './transport';

export async function listBaselines(projectId: number): Promise<Baseline[]> {
  return fetchJson<Baseline[]>(`/api/projects/${projectId}/baselines`);
}

export async function getBaseline(projectId: number, baselineId: number): Promise<Baseline> {
  return fetchJson<Baseline>(`/api/projects/${projectId}/baselines/${baselineId}`);
}

export async function createBaseline(
  projectId: number,
  name: string,
  description: string | null | undefined,
  csrfToken: string,
): Promise<Baseline> {
  return fetchJson<Baseline>(`/api/projects/${projectId}/baselines`, {
    method: 'POST',
    headers: {
      ...JSON_HEADERS,
      'X-CSRF-Token': csrfToken,
    },
    body: JSON.stringify({
      name,
      description: description?.trim() ? description.trim() : null,
    }),
  });
}

export async function getBaselineRequirements(
  projectId: number,
  baselineId: number,
): Promise<Requirement[]> {
  return fetchJson<Requirement[]>(
    `/api/projects/${projectId}/baselines/${baselineId}/requirements`,
  );
}

export async function getBaselineTraceability(
  projectId: number,
  baselineId: number,
): Promise<BaselineTraceabilityRow[]> {
  return fetchJson<BaselineTraceabilityRow[]>(
    `/api/projects/${projectId}/baselines/${baselineId}/traceability`,
  );
}

export async function getBaselineVerifications(
  projectId: number,
  baselineId: number,
): Promise<BaselineVerificationSnapshot[]> {
  return fetchJson<BaselineVerificationSnapshot[]>(
    `/api/projects/${projectId}/baselines/${baselineId}/verifications`,
  );
}
