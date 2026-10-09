import { useDashboard } from '@/context/DashboardContext';

/** Whether the project is archived, i.e. read-only for everyone (issue #381). */
export function useProjectArchived(projectId: number | null | undefined): boolean {
  const projects = useDashboard().dashboard?.projects ?? [];
  return projectId != null && projects.some((p) => p.id === projectId && p.archived === true);
}
