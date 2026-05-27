import { useCallback, useEffect, useMemo, useState } from 'react';
import { useSearchParams } from 'react-router-dom';
import ReactFlow, {
  Background,
  Controls,
  type Edge,
  type Node,
  type NodeMouseHandler,
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
import ResizableGraphShell from '@/components/graph/ResizableGraphShell';
import { useGraphNodeNavigation } from '@/hooks/useGraphNodeNavigation';
import { highlightEdgesForSelection, highlightNodesForSelection } from '@/utils/graphHighlight';

export type HierarchyKind = 'reqs' | 'vers' | 'both';

const KIND_PARAM = 'kind';
const VALID_KINDS: HierarchyKind[] = ['reqs', 'vers', 'both'];

function readKind(value: string | null): HierarchyKind {
  return (VALID_KINDS as string[]).includes(value ?? '') ? (value as HierarchyKind) : 'both';
}

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
  kind: HierarchyKind = 'both',
): { nodes: Node[]; edges: Edge[] } {
  const reqById = new Map(requirements.map((r) => [r.id, r]));
  const verById = new Map(verifications.map((v) => [v.id, v]));

  const includeReqs = kind === 'reqs' || kind === 'both';
  const includeVers = kind === 'vers' || kind === 'both';

  const reqIds = new Set<number>();
  if (includeReqs) {
    for (const r of requirements) {
      const parents = parentIdsOf(r);
      if (parents.length === 0) continue;
      reqIds.add(r.id);
      for (const pid of parents) reqIds.add(pid);
    }
  }

  const verIds = new Set<number>();
  if (includeVers) {
    for (const v of verifications) {
      if (v.parent_id == null) continue;
      verIds.add(v.id);
      verIds.add(v.parent_id);
    }
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
  if (includeReqs) {
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
  }
  if (includeVers) {
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
  }

  return { nodes, edges };
}

function emptyMessage(kind: HierarchyKind): string {
  switch (kind) {
    case 'reqs':
      return 'No requirement parent links yet.';
    case 'vers':
      return 'No verification parent links yet.';
    default:
      return 'No parent links between requirements or verifications yet.';
  }
}

export default function HierarchyGraph({
  projectId,
  basePath,
}: {
  projectId: number;
  basePath: string;
}) {
  const [requirements, setRequirements] = useState<Requirement[]>([]);
  const [verifications, setVerifications] = useState<Verification[]>([]);
  const [loading, setLoading] = useState(true);
  const [err, setErr] = useState<string | null>(null);
  const [searchParams, setSearchParams] = useSearchParams();
  const kind = readKind(searchParams.get(KIND_PARAM));

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

  const setKind = (next: HierarchyKind) => {
    const params = new URLSearchParams(searchParams);
    if (next === 'both') {
      params.delete(KIND_PARAM);
    } else {
      params.set(KIND_PARAM, next);
    }
    setSearchParams(params, { replace: true });
  };

  const { nodes: baseNodes, edges: baseEdges } = useMemo(
    () => buildHierarchyGraph(requirements, verifications, kind),
    [requirements, verifications, kind],
  );

  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);

  useEffect(() => {
    setSelectedNodeId(null);
  }, [kind, projectId]);

  useEffect(() => {
    if (selectedNodeId != null && !baseNodes.some((n) => n.id === selectedNodeId)) {
      setSelectedNodeId(null);
    }
  }, [baseNodes, selectedNodeId]);

  const nodes = useMemo(
    () => highlightNodesForSelection(baseNodes, baseEdges, selectedNodeId),
    [baseNodes, baseEdges, selectedNodeId],
  );

  const edges = useMemo(
    () => highlightEdgesForSelection(baseEdges, selectedNodeId),
    [baseEdges, selectedNodeId],
  );

  const onNodeClick: NodeMouseHandler = useCallback((_, node) => {
    setSelectedNodeId(node.id);
  }, []);

  const { onNodeDoubleClick } = useGraphNodeNavigation(basePath);

  const empty = baseNodes.length === 0 && !loading && !err;

  return (
    <div>
      <KindFilter kind={kind} onChange={setKind} />
      {err ? (
        <div className="p-4 rounded-xl bg-red-500/10 border border-red-500/25 text-red-200 text-sm">
          {err}
        </div>
      ) : loading ? (
        <ResizableGraphShell projectId={projectId} viewKey="hierarchy" placeholder>
          <span className="text-stitch-muted text-sm">Loading hierarchy graph…</span>
        </ResizableGraphShell>
      ) : empty ? (
        <ResizableGraphShell projectId={projectId} viewKey="hierarchy" placeholder>
          <div className="flex flex-col items-center text-stitch-muted text-sm">
            {emptyMessage(kind)}
            <button
              type="button"
              onClick={() => void load()}
              className="mt-3 text-stitch-accent text-xs font-semibold hover:underline"
            >
              Retry
            </button>
          </div>
        </ResizableGraphShell>
      ) : (
        <ResizableGraphShell projectId={projectId} viewKey="hierarchy">
          <ReactFlow
            nodes={nodes}
            edges={edges}
            nodeTypes={nodeTypes}
            fitView
            fitViewOptions={{ padding: 0.15, minZoom: 0.05 }}
            minZoom={0.05}
            maxZoom={2}
            nodesDraggable={false}
            selectNodesOnDrag={false}
            zoomOnDoubleClick={false}
            proOptions={{ hideAttribution: true }}
            onNodeClick={onNodeClick}
            onNodeDoubleClick={onNodeDoubleClick}
            onPaneClick={() => setSelectedNodeId(null)}
          >
            <Background color="rgba(255,255,255,0.06)" gap={20} />
            <Controls />
          </ReactFlow>
        </ResizableGraphShell>
      )}
    </div>
  );
}

function KindFilter({
  kind,
  onChange,
}: {
  kind: HierarchyKind;
  onChange: (next: HierarchyKind) => void;
}) {
  const seg = 'flex items-center gap-2 px-3 py-1.5 text-xs font-bold rounded-md transition-colors';
  const inactive = 'text-stitch-muted hover:bg-stitch-higher hover:text-stitch-fg';
  const active = 'bg-stitch-elevated text-stitch-accent shadow-stitch-inset border border-stitch-border';
  return (
    <div
      role="group"
      aria-label="Hierarchy filter"
      className="flex p-1 bg-stitch-surface rounded-lg gap-1 border border-stitch-border w-fit mb-4"
    >
      <button
        type="button"
        aria-pressed={kind === 'reqs'}
        onClick={() => onChange('reqs')}
        className={`${seg} ${kind === 'reqs' ? active : inactive}`}
      >
        <span className="material-symbols-outlined text-sm">list_alt</span>
        Requirements
      </button>
      <button
        type="button"
        aria-pressed={kind === 'vers'}
        onClick={() => onChange('vers')}
        className={`${seg} ${kind === 'vers' ? active : inactive}`}
      >
        <span className="material-symbols-outlined text-sm">verified</span>
        Verifications
      </button>
      <button
        type="button"
        aria-pressed={kind === 'both'}
        onClick={() => onChange('both')}
        className={`${seg} ${kind === 'both' ? active : inactive}`}
      >
        <span className="material-symbols-outlined text-sm">hub</span>
        Both
      </button>
    </div>
  );
}
