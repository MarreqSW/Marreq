import { useCallback, useEffect, useMemo, useState } from 'react';
import ReactFlow, {
  Background,
  Controls,
  Handle,
  Position,
  type Edge,
  type Node,
  type NodeProps,
  type NodeTypes,
} from 'reactflow';
import 'reactflow/dist/style.css';
import { listMatrix, listRequirements, listVerifications } from '@/api/client';
import type { MatrixLink, Requirement, Verification } from '@/api/types';

type ReqNodeData = { kind: 'requirement'; id: string; label: string; statusLine: string };
type VerNodeData = { kind: 'verification'; id: string; label: string; ref: string };

function RequirementFlowNode({ data }: NodeProps<ReqNodeData>) {
  const verified = /verified|accepted|approved/i.test(data.statusLine);
  return (
    <div className="px-4 py-3 rounded-md bg-stitch-elevated border border-stitch-border w-48 relative shadow-stitch">
      <Handle
        type="target"
        position={Position.Left}
        className="!bg-stitch-muted !w-2 !h-2 !border-stitch-border"
      />
      <div className="text-[10px] font-bold text-stitch-accent uppercase mb-1 tracking-tighter">
        {data.id}
      </div>
      <div className="text-xs font-semibold text-white leading-tight">{data.label}</div>
      <div
        className={`mt-2 h-1 w-full rounded-full ${verified ? 'bg-emerald-400/80' : 'bg-white/20'}`}
      />
      <Handle
        type="source"
        position={Position.Right}
        className="!bg-stitch-accent !w-2 !h-2 !border-0"
      />
    </div>
  );
}

function VerificationFlowNode({ data }: NodeProps<VerNodeData>) {
  return (
    <div className="px-3 py-2 rounded-md bg-stitch-surface border border-stitch-accent/35 w-44 relative shadow-stitch">
      <Handle
        type="target"
        position={Position.Left}
        className="!bg-stitch-accent/60 !w-2 !h-2"
      />
      <div className="text-[10px] font-mono text-stitch-muted">{data.ref}</div>
      <div className="text-xs font-medium text-white/95 leading-tight">{data.label}</div>
      <Handle
        type="source"
        position={Position.Right}
        className="!bg-stitch-muted !w-2 !h-2"
      />
    </div>
  );
}

const nodeTypes: NodeTypes = {
  requirement: RequirementFlowNode,
  verification: VerificationFlowNode,
};

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
      position: { x: 40, y: 40 + yi * 120 },
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
      position: { x: 420, y: 40 + yj * 100 },
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
