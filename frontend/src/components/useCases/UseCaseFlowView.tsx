import { useMemo } from 'react';
import ReactFlow, {
  Background,
  Controls,
  Handle,
  MarkerType,
  Position,
  type Edge,
  type Node,
  type NodeProps,
} from 'reactflow';
import 'reactflow/dist/style.css';
import type { UseCaseDocument, UseCaseLabels } from '@/api/types';

/** Flow view of a use case (screen 9 of #388, issue #403), derived from the document and its labels. */

const COLUMN = 380;
const ROW = 96;
const NODE_WIDTH = 260;

type FlowNodeData = {
  /** `start`, `end`, `step` (main), `branch` (a branch's title), `branch-step`, `terminal`. */
  kind: 'start' | 'end' | 'step' | 'branch' | 'branch-step' | 'terminal';
  label: string;
  text: string;
  /** Alternative or exception, for branch nodes and their steps. */
  tone?: 'alternative' | 'exception';
  /** Element (or branch) id in the document; absent for start and end. */
  elementId?: string;
  selected?: boolean;
};

function FlowNode({ data }: NodeProps<FlowNodeData>) {
  const tone =
    data.tone === 'exception'
      ? 'border-rose-600'
      : data.tone === 'alternative'
        ? 'border-sky-600'
        : 'border-stitch-accent';
  const ring = data.selected ? 'ring-2 ring-amber-500' : '';
  if (data.kind === 'start' || data.kind === 'end' || data.kind === 'terminal') {
    return (
      <div
        className={`rounded-full border-2 bg-stitch-surface px-4 py-1.5 text-center text-xs font-semibold text-stitch-fg shadow-sm ${
          data.kind === 'terminal' && data.text.includes('without') ? 'border-rose-600' : 'border-emerald-600'
        }`}
        style={{ width: NODE_WIDTH }}
      >
        <Handle type="target" id="top" position={Position.Top} className="opacity-0" />
        {data.text}
        <Handle type="source" id="bottom" position={Position.Bottom} className="opacity-0" />
      </div>
    );
  }
  return (
    <div
      className={`rounded-md border border-l-4 bg-stitch-surface px-3 py-2 text-left shadow-sm ${tone} ${ring}`}
      style={{ width: NODE_WIDTH }}
    >
      <Handle type="target" id="top" position={Position.Top} className="opacity-0" />
      <Handle type="target" id="left" position={Position.Left} className="opacity-0" />
      <Handle type="target" id="right-in" position={Position.Right} className="opacity-0" />
      <Handle type="source" id="left-out" position={Position.Left} className="opacity-0" />
      <div className="mb-0.5 flex items-center gap-1.5">
        <span className="font-mono text-[11px] font-bold text-stitch-fg">{data.label}</span>
        {data.kind === 'branch' ? (
          <span
            className={`rounded-sm px-1 text-[9px] font-bold uppercase tracking-wider ${
              data.tone === 'exception'
                ? 'bg-rose-500/15 text-rose-800 dark:text-rose-300'
                : 'bg-sky-500/15 text-sky-800 dark:text-sky-300'
            }`}
          >
            {data.tone}
          </span>
        ) : null}
      </div>
      <p className={`line-clamp-3 text-xs text-stitch-fg ${data.kind === 'branch' ? 'font-semibold' : ''}`}>
        {data.text}
      </p>
      <Handle type="source" id="bottom" position={Position.Bottom} className="opacity-0" />
      <Handle type="source" id="right" position={Position.Right} className="opacity-0" />
    </div>
  );
}

const nodeTypes = { useCase: FlowNode };

const MAIN_EDGE = { stroke: 'var(--color-stitch-accent)', strokeWidth: 2 };

/**
 * Nodes and edges for a use case: the main scenario top to bottom between a
 * start and an end; the branches in one column beside it, each starting just
 * below its origin step (or below the branch before it, so that no edge runs
 * through a node), then rejoining (dashed, back to the main column) or ending.
 */
export function buildUseCaseFlow(
  doc: UseCaseDocument,
  labels: UseCaseLabels,
  selected: string | null = null,
): { nodes: Node<FlowNodeData>[]; edges: Edge[] } {
  const nodes: Node<FlowNodeData>[] = [];
  const edges: Edge[] = [];
  const node = (id: string, x: number, y: number, data: FlowNodeData) =>
    nodes.push({ id, type: 'useCase', position: { x, y }, data, draggable: false });
  const arrow = { type: MarkerType.ArrowClosed };
  // Every edge names its handles: a node has several of each kind.
  const down = { sourceHandle: 'bottom', targetHandle: 'top' };

  node('start', 0, 0, {
    kind: 'start',
    label: '',
    text: doc.preconditions.length
      ? `Start: ${doc.preconditions.map((p) => p.text).join(' · ')}`
      : 'Start',
  });
  const rowOf = new Map<string, number>();
  let previous = 'start';
  doc.main.forEach((step, i) => {
    const row = i + 1;
    rowOf.set(step.id, row);
    node(step.id, 0, row * ROW, {
      kind: 'step',
      label: labels[step.id] ?? String(i + 1),
      text: step.text,
      elementId: step.id,
      selected: selected === step.id,
    });
    edges.push({ id: `${previous}->${step.id}`, source: previous, target: step.id, ...down, markerEnd: arrow, style: MAIN_EDGE });
    previous = step.id;
  });
  const endRow = doc.main.length + 1;
  node('end', 0, endRow * ROW, {
    kind: 'end',
    label: '',
    text: doc.postconditions.length
      ? `End: ${doc.postconditions.map((p) => p.text).join(' · ')}`
      : 'End',
  });
  edges.push({ id: `${previous}->end`, source: previous, target: 'end', ...down, markerEnd: arrow, style: MAIN_EDGE });

  // Rows of the branch column are taken top to bottom.
  let nextFree = 0;
  doc.branches.forEach((branch) => {
    const x = COLUMN;
    const originRow = Math.max((rowOf.get(branch.from_step) ?? 1) + 0.5, nextFree) - 0.5;
    const tone = branch.kind;
    const color = tone === 'exception' ? '#e11d48' : '#0284c7';
    const branchSelected = selected === branch.id;
    node(branch.id, x, originRow * ROW + ROW / 2, {
      kind: 'branch',
      label: labels[branch.id] ?? '',
      text: branch.title,
      tone,
      elementId: branch.id,
      selected: branchSelected,
    });
    edges.push({
      id: `${branch.from_step}->${branch.id}`,
      source: branch.from_step,
      sourceHandle: 'right',
      target: branch.id,
      targetHandle: 'left',
      markerEnd: arrow,
      style: { stroke: color, strokeWidth: 1.5 },
    });
    let last = branch.id;
    branch.steps.forEach((step, j) => {
      node(step.id, x, (originRow + j + 1) * ROW + ROW / 2, {
        kind: 'branch-step',
        label: labels[step.id] ?? '',
        text: step.text,
        tone,
        elementId: step.id,
        selected: selected === step.id || branchSelected,
      });
      edges.push({ id: `${last}->${step.id}`, source: last, target: step.id, ...down, markerEnd: arrow, style: { stroke: color, strokeWidth: 1.5 } });
      last = step.id;
    });
    const lastRow = originRow + branch.steps.length + 0.5;
    if (branch.end.type === 'rejoin') {
      nextFree = lastRow + 1;
      edges.push({
        id: `${last}->rejoin:${branch.end.step}`,
        source: last,
        sourceHandle: 'left-out',
        target: branch.end.step,
        targetHandle: 'right-in',
        markerEnd: arrow,
        label: `continues at ${labels[branch.end.step] ?? '?'}`,
        style: { stroke: color, strokeWidth: 1.5, strokeDasharray: '6 4' },
        labelStyle: { fontSize: 10, fill: 'var(--color-stitch-fg)' },
        labelBgStyle: { fill: 'var(--color-stitch-surface)' },
      });
    } else {
      nextFree = lastRow + 2;
      const terminal = `${branch.id}:end`;
      node(terminal, x, (originRow + branch.steps.length + 1) * ROW + ROW / 2, {
        kind: 'terminal',
        label: '',
        text: branch.end.outcome === 'success' ? 'Use case ends' : 'Use case ends without success',
      });
      edges.push({ id: `${last}->${terminal}`, source: last, target: terminal, ...down, markerEnd: arrow, style: { stroke: color, strokeWidth: 1.5 } });
    }
  });
  return { nodes, edges };
}

type Props = {
  doc: UseCaseDocument;
  labels: UseCaseLabels;
  /** The selected element or branch, shared with the document view. */
  selected: string | null;
  onSelect: (elementId: string | null) => void;
};

export default function UseCaseFlowView({ doc, labels, selected, onSelect }: Props) {
  const { nodes, edges } = useMemo(() => buildUseCaseFlow(doc, labels, selected), [doc, labels, selected]);
  return (
    <div
      className="h-[640px] min-w-0 flex-1 overflow-hidden rounded-lg border border-stitch-border bg-stitch-canvas shadow-sm"
      data-testid="use-case-flow"
    >
      <ReactFlow
        nodes={nodes}
        edges={edges}
        nodeTypes={nodeTypes}
        fitView
        fitViewOptions={{ padding: 0.15 }}
        nodesConnectable={false}
        elementsSelectable
        onNodeClick={(_, node) => onSelect((node.data as FlowNodeData).elementId ?? null)}
        onPaneClick={() => onSelect(null)}
        proOptions={{ hideAttribution: true }}
      >
        <Background gap={24} size={1} />
        <Controls showInteractive={false} />
      </ReactFlow>
    </div>
  );
}
