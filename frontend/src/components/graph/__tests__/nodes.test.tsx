import { render, screen } from '@testing-library/react';
import type { NodeProps } from 'reactflow';
import { ReactFlowProvider } from 'reactflow';
import { describe, expect, it } from 'vitest';
import {
  RequirementFlowNode,
  VerificationFlowNode,
  type ReqNodeData,
  type VerNodeData,
} from '../nodes';

function props<T>(data: T): NodeProps<T> {
  return {
    id: 'n1',
    data,
    type: 'x',
    selected: false,
    isConnectable: false,
    zIndex: 0,
    xPos: 0,
    yPos: 0,
    dragging: false,
  } as NodeProps<T>;
}

// Issue #376: node titles were hard-coded white, unreadable in the light theme.
describe('graph nodes', () => {
  it('render titles in the theme foreground colour', () => {
    render(
      <ReactFlowProvider>
        <RequirementFlowNode
          {...props<ReqNodeData>({
            kind: 'requirement',
            id: 'REQ-1',
            label: 'Power budget',
            statusLine: 'Draft',
          })}
        />
        <VerificationFlowNode
          {...props<VerNodeData>({
            kind: 'verification',
            id: 'v-1',
            label: 'Thermal vacuum test',
            ref: 'VER-1',
          })}
        />
      </ReactFlowProvider>,
    );
    for (const title of ['Power budget', 'Thermal vacuum test']) {
      const el = screen.getByText(title);
      expect(el).toHaveClass('text-stitch-fg');
      expect(el.className).not.toMatch(/text-white/);
    }
  });
});
