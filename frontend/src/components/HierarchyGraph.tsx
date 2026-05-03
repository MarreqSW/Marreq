import { useCallback, useEffect, useMemo, useState } from 'react';
import ReactFlow, {
  Background,
  Controls,
  type Edge,
  type Node,
} from 'reactflow';
import 'reactflow/dist/style.css';
import { listRequirements, listVerifications } from '@/api/client';
import type { Requirement, Verification } from '@/api/types';
import {
  COLUMN_X,
  ROW_HEIGHT,
  TOP_PADDING,
  nodeTypes,
} from '@/components/graph/nodes';

/** Read upstream parent ids from the list API, falling back to the legacy `parent_id`. */
function parentIdsOf(req: Requirement): number[] {
  if (req.parent_requirement_ids && req.parent_requirement_ids.length > 0) {
    return req.parent_requirement_ids;
  }
  if (req.parent_id != null) return [req.parent_id];
  return [];
}

export function buildHierarchyGraph(
  requirements: Requirement[],
  verifications: Verification[],
): { nodes: Node[]; edges: Edge[] } {
  const reqById = new Map(requirements.map((r) => [r.id, r]));
  const verById = new Map(verifications.map((v) => [v.id, v]));

  // Requirement participation set: any req that has a parent or is a parent of someone.
  const reqIds = new Set<number>();
  for (const r of requirements) {
    const parents = parentIdsOf(r);
    if (parents.length === 0) continue;
    reqIds.add(r.id);
    for (const pid of parents) reqIds.add(pid);
  }

  // Verification participation set: any ver that has parent_id or is a parent.
  const verIds = new Set<number>();
  for (const v of verifications) {
    if (v.parent_id == null) continue;
    verIds.add(v.id);
    verIds.add(v.parent_id);
  }

  const nodes: Node[] = [];
  let yi = 0;
  for (const rid of reqIds) {
    const r = reqById.get(rid);
    nodes.push({
      id: `r-${rid}`,
      type: 'requirement',
      position: { x: COLUMN_X.requirement, y: TOP_PADDING + yi * ROW_HEIGHT.requirement },
      data: {
        kind: 'requirement',
        id: r?.reference_code ?? `REQ-${rid}`,
        label: r?.title ?? `Requirement ${rid}`,
        statusLine: r?.approval_state ?? '',
      },
    });
    yi += 1;
  }

  let yj = 0;
  for (const vid of verIds) {
    const v = verById.get(vid);
    nodes.push({
      id: `v-${vid}`,
      type: 'verification',
      position: { x: COLUMN_X.verification, y: TOP_PADDING + yj * ROW_HEIGHT.verification },
      data: {
        kind: 'verification',
        id: String(vid),
        label: v?.name ?? `Verification ${vid}`,
        ref: v?.reference_code ?? `VER-${vid}`,
      },
    });
    yj += 1;
  }

  const edges: Edge[] = [];
  let ei = 0;
  for (const r of requirements) {
    for (const pid of parentIdsOf(r)) {
      if (!reqIds.has(pid) || !reqIds.has(r.id)) continue;
      edges.push({
        id: `er-${ei++}`,
        source: `r-${pid}`,
        target: `r-${r.id}`,
        style: { stroke: '#8ab4f8', strokeWidth: 1.5 },
      });
    }
  }
  for (const v of verifications) {
    if (v.parent_id == null) continue;
    if (!verIds.has(v.parent_id) || !verIds.has(v.id)) continue;
    edges.push({
      id: `ev-${ei++}`,
      source: `v-${v.parent_id}`,
      target: `v-${v.id}`,
      style: { stroke: '#a6f0c6', strokeWidth: 1.5, strokeDasharray: '4 4' },
    });
  }

  return { nodes, edges };
}

export default function HierarchyGraph({ projectId }: { projectId: number }) {
  const [requirements, setRequirements] = useState<Requirement[]>([]);
  const [verifications, setVerifications] = useState<Verification[]>([]);
  const [loading, setLoading] = useState(true);
  const [err, setErr] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setErr(null);
    try {
      const [reqs, allVer] = await Promise.all([
        listRequirements(projectId),
        listVerifications(),
      ]);
      setRequirements(reqs);
      setVerifications(allVer.filter((v) => v.project_id === projectId));
    } catch (e) {
      setErr(e instanceof Error ? e.message : 'Failed to load hierarchy graph');
    } finally {
      setLoading(false);
    }
  }, [projectId]);

  useEffect(() => {
    void load();
  }, [load]);

  const { nodes, edges } = useMemo(
    () => buildHierarchyGraph(requirements, verifications),
    [requirements, verifications],
  );

  const empty = nodes.length === 0 && !loading && !err;

  if (loading) {
    return (
      <div className="h-[600px] flex items-center justify-center border border-stitch-border rounded-xl bg-stitch-surface text-stitch-muted text-sm">
        Loading hierarchy graph…
      </div>
    );
  }

  if (err) {
    return (
      <div className="p-4 rounded-xl bg-red-500/10 border border-red-500/25 text-red-200 text-sm">
        {err}
      </div>
    );
  }

  if (empty) {
    return (
      <div className="h-[400px] flex flex-col items-center justify-center border border-dashed border-stitch-border rounded-xl bg-stitch-surface/50 text-stitch-muted text-sm">
        No parent links between requirements or verifications yet.
        <button
          type="button"
          onClick={() => void load()}
          className="mt-3 text-stitch-accent text-xs font-semibold hover:underline"
        >
          Retry
        </button>
      </div>
    );
  }

  return (
    <div className="stitch-flow h-[600px] w-full border border-stitch-border rounded-xl bg-stitch-surface overflow-hidden shadow-stitch">
      <ReactFlow
        nodes={nodes}
        edges={edges}
        nodeTypes={nodeTypes}
        fitView
        proOptions={{ hideAttribution: true }}
      >
        <Background color="rgba(255,255,255,0.06)" gap={20} />
        <Controls />
      </ReactFlow>
    </div>
  );
}
