import { useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import type { NodeMouseHandler } from 'reactflow';
import { entityDetailPath, parseGraphNodeId } from '@/utils/graphNavigation';

export function useGraphNodeNavigation(basePath: string): {
  onNodeDoubleClick: NodeMouseHandler;
} {
  const navigate = useNavigate();

  const onNodeDoubleClick: NodeMouseHandler = useCallback(
    (event, node) => {
      event.stopPropagation();
      const parsed = parseGraphNodeId(node.id);
      if (!parsed) return;
      navigate(entityDetailPath(basePath, parsed.kind, parsed.entityId));
    },
    [basePath, navigate],
  );

  return { onNodeDoubleClick };
}
