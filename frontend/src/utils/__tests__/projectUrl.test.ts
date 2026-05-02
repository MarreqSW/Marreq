import { describe, it, expect } from 'vitest';
import { projectPath, projectPathFromSlug } from '../projectUrl';
import type { DashboardProject } from '@/api/types';

function project(overrides: Partial<DashboardProject> = {}): DashboardProject {
  return {
    id: 1,
    name: 'Demo',
    slug: 'demo',
    project_base_path: '',
    group_id: null,
    group_name: null,
    group_slug: null,
    ...overrides,
  };
}

describe('projectPath', () => {
  it('uses project_base_path when available', () => {
    const p = project({ project_base_path: '/orgs/acme/demo', slug: 'demo' });
    expect(projectPath(p, 'matrix')).toBe('/orgs/acme/demo/matrix');
  });

  it('falls back to /<slug> when base path is empty string', () => {
    const p = project({ project_base_path: '', slug: 'widgets' });
    expect(projectPath(p, 'verifications')).toBe('/widgets/verifications');
  });
});

describe('projectPathFromSlug', () => {
  it('prefixes a leading slash when slug has none', () => {
    expect(projectPathFromSlug('demo', 'matrix')).toBe('/demo/matrix');
  });

  it('keeps an existing leading slash', () => {
    expect(projectPathFromSlug('/orgs/acme/demo', 'matrix')).toBe('/orgs/acme/demo/matrix');
  });
});
