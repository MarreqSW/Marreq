import type { DashboardPayload, DashboardPayloadWire, DashboardProject } from './types';
import { fetchJson } from './transport';

function normalizeDashboard(wire: DashboardPayloadWire): DashboardPayload {
  const projects: DashboardProject[] = wire.projects.map((p) => ({
    ...p,
    id: p.project_id,
    slug: p.project_slug,
    project_base_path: p.project_base_path ?? `/${p.project_slug}`,
    group_id: p.group_id ?? null,
    group_name: p.group_name ?? null,
    group_slug: p.group_slug ?? null,
  }));
  return {
    ...wire,
    projects,
  };
}

export async function getDashboard(): Promise<DashboardPayload> {
  const wire = await fetchJson<DashboardPayloadWire>('/api/dashboard');
  return normalizeDashboard(wire);
}
