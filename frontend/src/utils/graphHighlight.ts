import type { Edge, Node } from 'reactflow';

/**
 * All node ids in the same weakly connected component as `startId`
 * (edges treated as undirected).
 */
export function connectedComponentNodeIds(startId: string, edges: Edge[]): Set<string> {
  const adj = new Map<string, Set<string>>();
  for (const e of edges) {
    if (!adj.has(e.source)) adj.set(e.source, new Set());
    if (!adj.has(e.target)) adj.set(e.target, new Set());
    adj.get(e.source)!.add(e.target);
    adj.get(e.target)!.add(e.source);
  }
  const seen = new Set<string>();
  const stack = [startId];
  seen.add(startId);
  while (stack.length > 0) {
    const id = stack.pop()!;
    for (const nb of adj.get(id) ?? []) {
      if (!seen.has(nb)) {
        seen.add(nb);
        stack.push(nb);
      }
    }
  }
  return seen;
}

/** Dim nodes outside the selected node's connected component; ring the clicked node. */
export function highlightNodesForSelection(nodes: Node[], edges: Edge[], selectedId: string | null): Node[] {
  const related = selectedId ? connectedComponentNodeIds(selectedId, edges) : null;
  return nodes.map((n) => {
    const isSel = Boolean(selectedId && n.id === selectedId);
    return {
      ...n,
      selected: isSel,
      zIndex: isSel ? 1000 : (n.zIndex ?? 0),
      data: {
        ...n.data,
        dimmed: Boolean(related && !related.has(n.id)),
        selected: isSel,
      },
    };
  });
}

/** Fade edges that are not wholly inside the selected connected component. */
export function highlightEdgesForSelection(edges: Edge[], selectedId: string | null): Edge[] {
  const related = selectedId ? connectedComponentNodeIds(selectedId, edges) : null;
  return edges.map((e) => {
    const baseStyle =
      typeof e.style === 'object' && e.style !== null && !Array.isArray(e.style)
        ? { ...(e.style as Record<string, unknown>) }
        : {};
    const active =
      related == null || (related.has(e.source) && related.has(e.target));
    return {
      ...e,
      style: { ...baseStyle, opacity: active ? 1 : 0.08 },
    };
  });
}
