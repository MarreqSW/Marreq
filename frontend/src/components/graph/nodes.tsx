import { Handle, Position, type NodeProps, type NodeTypes } from 'reactflow';

export type ReqNodeData = {
  kind: 'requirement';
  id: string;
  label: string;
  statusLine: string;
  dimmed?: boolean;
  selected?: boolean;
};

export type VerNodeData = {
  kind: 'verification';
  id: string;
  label: string;
  ref: string;
  dimmed?: boolean;
  selected?: boolean;
};

export function RequirementFlowNode({ data }: NodeProps<ReqNodeData>) {
  const verified = /verified|accepted|approved/i.test(data.statusLine);
  return (
    <div
      title="Double-click to open"
      className={`px-4 py-3 rounded-md bg-stitch-elevated border w-48 relative shadow-stitch cursor-pointer transition-[opacity,transform,box-shadow] duration-200 ${
        data.dimmed ? 'opacity-[0.14] saturate-[0.35]' : 'opacity-100'
      } ${
        data.selected
          ? 'border-stitch-accent ring-[3px] ring-stitch-accent ring-offset-2 ring-offset-stitch-surface scale-[1.03] shadow-[0_0_24px_rgba(138,180,248,0.55)] z-10'
          : 'border-stitch-border'
      }`}
    >
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

export function VerificationFlowNode({ data }: NodeProps<VerNodeData>) {
  return (
    <div
      title="Double-click to open"
      className={`px-3 py-2 rounded-md bg-stitch-surface border w-44 relative shadow-stitch cursor-pointer transition-[opacity,transform,box-shadow] duration-200 ${
        data.dimmed ? 'opacity-[0.14] saturate-[0.35]' : 'opacity-100'
      } ${
        data.selected
          ? 'border-stitch-accent ring-[3px] ring-stitch-accent ring-offset-2 ring-offset-stitch-surface scale-[1.03] shadow-[0_0_24px_rgba(138,180,248,0.55)] z-10'
          : 'border-stitch-accent/35'
      }`}
    >
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

export const nodeTypes: NodeTypes = {
  requirement: RequirementFlowNode,
  verification: VerificationFlowNode,
};

/** Constants used to lay out nodes in two side-by-side columns (req | ver). */
export const COLUMN_X = {
  requirement: 40,
  verification: 420,
} as const;

export const ROW_HEIGHT = {
  requirement: 120,
  verification: 100,
} as const;

export const TOP_PADDING = 40;
