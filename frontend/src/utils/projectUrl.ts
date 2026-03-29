import type { DashboardProject } from '@/api/types';

export function projectPath(project: DashboardProject, subpath: string): string {
  const base = project.project_base_path || `/${project.slug}`;
  return `${base}/${subpath}`;
}

export function projectPathFromSlug(slug: string, subpath: string): string {
  const base = slug.startsWith('/') ? slug : `/${slug}`;
  return `${base}/${subpath}`;
}
