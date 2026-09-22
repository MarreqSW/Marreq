export type GraphEntityKind = 'requirement' | 'verification';

export type ParsedGraphNodeId = {
  kind: GraphEntityKind;
  entityId: number;
};

/** Parse React Flow node id (`r-42`, `v-7`) into entity kind and numeric id. */
export function parseGraphNodeId(nodeId: string): ParsedGraphNodeId | null {
  const m = /^(r|v)-(\d+)$/.exec(nodeId);
  if (!m) return null;
  return {
    kind: m[1] === 'r' ? 'requirement' : 'verification',
    entityId: Number(m[2]),
  };
}

/** View page path for a requirement or verification (matches list View action). */
export function entityDetailPath(
  basePath: string,
  kind: GraphEntityKind,
  entityId: number,
): string {
  const base = basePath.replace(/\/$/, '');
  if (kind === 'requirement') {
    return `${base}/requirements/${entityId}`;
  }
  return `${base}/verifications/${entityId}`;
}

/** Create page for a child of the selected graph node. */
export function entityCreateChildPath(
  basePath: string,
  kind: GraphEntityKind,
  parentId: number,
): string {
  const base = basePath.replace(/\/$/, '');
  if (kind === 'requirement') {
    return `${base}/requirements/new?parent=${parentId}`;
  }
  return `${base}/verifications/new?parent=${parentId}`;
}
