import { describe, it, expect } from 'vitest';
import { buildHierarchyGraph } from '../HierarchyGraph';
import type { Requirement, Verification } from '@/api/types';

function req(id: number, opts: Partial<Requirement> = {}): Requirement {
  return {
    id,
    project_id: 1,
    current_version_id: id,
    title: `Req ${id}`,
    description: '',
    status_id: 1,
    author_id: 1,
    reviewer_id: 1,
    reference_code: `REQ-${id}`,
    category_id: 1,
    parent_id: null,
    creation_date: '2026-01-01T00:00:00Z',
    update_date: '2026-01-01T00:00:00Z',
    deadline_date: null,
    applicability_id: 1,
    justification: null,
    approval_state: 'draft',
    approved_by: null,
    approved_at: null,
    ...opts,
  };
}

function ver(id: number, opts: Partial<Verification> = {}): Verification {
  return {
    id,
    name: `Test ${id}`,
    reference_code: `TEST-${id}`,
    description: '',
    source: '',
    status_id: 1,
    parent_id: null,
    project_id: 1,
    verification_method_id: null,
    author_id: 1,
    reviewer_id: 1,
    ...opts,
  };
}

describe('buildHierarchyGraph', () => {
  it('emits requirement parent edges from parent_requirement_ids', () => {
    const reqs = [
      req(1),
      req(2, { parent_requirement_ids: [1] }),
      req(3, { parent_requirement_ids: [1, 2] }),
    ];
    const { nodes, edges } = buildHierarchyGraph(reqs, []);

    const nodeIds = nodes.map((n) => n.id).sort();
    expect(nodeIds).toEqual(['r-1', 'r-2', 'r-3']);

    const edgePairs = edges.map((e) => `${e.source}->${e.target}`).sort();
    expect(edgePairs).toEqual(['r-1->r-2', 'r-1->r-3', 'r-2->r-3']);
  });

  it('falls back to legacy parent_id when parent_requirement_ids is empty', () => {
    const reqs = [req(10), req(11, { parent_id: 10 })];
    const { edges } = buildHierarchyGraph(reqs, []);
    expect(edges).toHaveLength(1);
    expect(edges[0]!.source).toBe('r-10');
    expect(edges[0]!.target).toBe('r-11');
  });

  it('emits verification parent edges from verifications.parent_id', () => {
    const verifications = [ver(1), ver(2, { parent_id: 1 }), ver(3, { parent_id: 2 })];
    const { nodes, edges } = buildHierarchyGraph([], verifications);

    const nodeIds = nodes.map((n) => n.id).sort();
    expect(nodeIds).toEqual(['v-1', 'v-2', 'v-3']);

    const edgePairs = edges.map((e) => `${e.source}->${e.target}`).sort();
    expect(edgePairs).toEqual(['v-1->v-2', 'v-2->v-3']);
  });

  it('produces an empty graph when no parents are present', () => {
    const { nodes, edges } = buildHierarchyGraph([req(1), req(2)], [ver(1), ver(2)]);
    expect(nodes).toEqual([]);
    expect(edges).toEqual([]);
  });

  it('renders a placeholder node when a parent is not in the supplied list', () => {
    const reqs = [req(2, { parent_requirement_ids: [1] })];
    const { nodes, edges } = buildHierarchyGraph(reqs, []);
    expect(nodes.map((n) => n.id).sort()).toEqual(['r-1', 'r-2']);
    const orphanParent = nodes.find((n) => n.id === 'r-1')!;
    expect(orphanParent.data).toMatchObject({ id: 'REQ-1', label: 'Requirement 1' });
    expect(edges).toHaveLength(1);
    expect(edges[0]).toMatchObject({ source: 'r-1', target: 'r-2' });
  });

  describe("kind filter", () => {
    const reqs = [req(1), req(2, { parent_requirement_ids: [1] })];
    const verifications = [ver(10), ver(11, { parent_id: 10 })];

    it("kind='reqs' hides verification nodes/edges", () => {
      const { nodes, edges } = buildHierarchyGraph(reqs, verifications, 'reqs');
      expect(nodes.every((n) => n.id.startsWith('r-'))).toBe(true);
      expect(edges.every((e) => e.id.startsWith('er-'))).toBe(true);
      expect(nodes).toHaveLength(2);
      expect(edges).toHaveLength(1);
    });

    it("kind='vers' hides requirement nodes/edges", () => {
      const { nodes, edges } = buildHierarchyGraph(reqs, verifications, 'vers');
      expect(nodes.every((n) => n.id.startsWith('v-'))).toBe(true);
      expect(edges.every((e) => e.id.startsWith('ev-'))).toBe(true);
      expect(nodes).toHaveLength(2);
      expect(edges).toHaveLength(1);
    });

    it("kind='both' shows both forests", () => {
      const { nodes, edges } = buildHierarchyGraph(reqs, verifications, 'both');
      expect(nodes).toHaveLength(4);
      expect(edges).toHaveLength(2);
      expect(nodes.some((n) => n.id.startsWith('r-'))).toBe(true);
      expect(nodes.some((n) => n.id.startsWith('v-'))).toBe(true);
    });

    it("'both' is the default when no kind argument is given", () => {
      const a = buildHierarchyGraph(reqs, verifications);
      const b = buildHierarchyGraph(reqs, verifications, 'both');
      expect(a.nodes.length).toBe(b.nodes.length);
      expect(a.edges.length).toBe(b.edges.length);
    });
  });

  it('uses distinct edge styles for requirement vs verification parent edges', () => {
    const reqs = [req(1), req(2, { parent_requirement_ids: [1] })];
    const verifications = [ver(10), ver(11, { parent_id: 10 })];
    const { edges } = buildHierarchyGraph(reqs, verifications);

    const reqEdge = edges.find((e) => e.id.startsWith('er-'))!;
    const verEdge = edges.find((e) => e.id.startsWith('ev-'))!;
    expect(reqEdge.style).toMatchObject({ stroke: '#8ab4f8' });
    expect(verEdge.style).toMatchObject({ stroke: '#a6f0c6', strokeDasharray: '4 4' });
  });
});
