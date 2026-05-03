import { useCallback, useEffect, useMemo, useState } from 'react';
import ReactFlow, {
  Background,
  Controls,
  type Edge,
  type Node,
} from 'reactflow';
import 'reactflow/dist/style.css';
import { listMatrix, listRequirements, listVerifications } from '@/api/client';
import type { MatrixLink, Requirement, Verification } from '@/api/types';
import {
  COLUMN_X,
  ROW_HEIGHT,
  TOP_PADDING,
  nodeTypes,
} from '@/components/graph/nodes';

function buildGraph(
  matrix: MatrixLink[],
  requirements: Requirement[],
  verifications: Verification[],
  projectId: number,
): { nodes: Node[]; edges: Edge[] } {
  const reqById = new Map(requirements.map((r) => [r.id, r]));
  const verById = new Map(verifications.map((v) => [v.id, v]));

  const reqIds = new Set<number>();
  const verIds = new Set<number>();
  for (const m of matrix) {
    if (m.project_id !== projectId) continue;
    reqIds.add(m.req_id);
    verIds.add(m.verification_id);
  }

  const nodes: Node[] = [];
  let yi = 0;
  for (const rid of reqIds) {
    const r = reqById.get(rid);
    const label = r?.title ?? `Requirement ${rid}`;
    const idStr = r?.reference_code ?? `REQ-${rid}`;
    nodes.push({
      id: `r-${rid}`,
      type: 'requirement',
      position: { x: COLUMN_X.requirement, y: TOP_PADDING + yi * ROW_HEIGHT.requirement },
      data: {
        kind: 'requirement',
        id: idStr,
        label,
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
  for (const m of matrix) {
    if (m.project_id !== projectId) continue;
    if (!reqIds.has(m.req_id) || !verIds.has(m.verification_id)) continue;
    edges.push({
      id: `e-${ei++}`,
      source: `r-${m.req_id}`,
      target: `v-${m.verification_id}`,
      animated: m.suspect,
      style: m.suspect ? { stroke: '#f28b82', strokeWidth: 2 } : { stroke: '#8ab4f8', strokeWidth: 1.5 },
    });
  }

  return { nodes, edges };
}

export default function TraceabilityGraph({ projectId }: { projectId: number }) {
  const [nodes, setNodes] = useState<Node[]>([]);
  const [edges, setEdges] = useState<Edge[]>([]);
  const [loading, setLoading] = useState(true);
  const [err, setErr] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setErr(null);
    try {
      const [matrix, requirements, allVer] = await Promise.all([
        listMatrix(projectId),
        listRequirements(projectId),
        listVerifications(),
      ]);
      const verifications = allVer.filter((v) => v.project_id === projectId);
      const { nodes: n, edges: e } = buildGraph(matrix, requirements, verifications, projectId);
      setNodes(n);
      setEdges(e);
    } catch (e) {
      setErr(e instanceof Error ? e.message : 'Failed to load graph');
    } finally {
      setLoading(false);
    }
  }, [projectId]);

  useEffect(() => {
    void load();
  }, [load]);

  const empty = useMemo(() => nodes.length === 0 && !loading && !err, [nodes.length, loading, err]);

  if (loading) {
    return (
      <div className="h-[600px] flex items-center justify-center border border-stitch-border rounded-xl bg-stitch-surface text-stitch-muted text-sm">
        Loading traceability graph…
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
        No matrix links in this project yet.
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
