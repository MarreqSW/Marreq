import { describe, expect, it } from 'vitest';

import {
  connectedComponentNodeIds,
  highlightEdgesForSelection,
  highlightNodesForSelection,
} from '../graphHighlight';

describe('connectedComponentNodeIds', () => {
  it('includes nodes reachable via undirected edges', () => {
    const edges = [
      { id: 'e1', source: 'a', target: 'b' },
      { id: 'e2', source: 'c', target: 'b' },
    ];
    expect(connectedComponentNodeIds('a', edges)).toEqual(new Set(['a', 'b', 'c']));
  });

  it('visits targets first when stack pops last-in', () => {
    const edges = [{ id: 'e1', source: 'x', target: 'y' }];
    expect(connectedComponentNodeIds('y', edges)).toEqual(new Set(['x', 'y']));
  });

  it('isolates disconnected components', () => {
    const edges = [
      { id: 'e1', source: 'a', target: 'b' },
      { id: 'e2', source: 'x', target: 'y' },
    ];
    expect(connectedComponentNodeIds('a', edges)).toEqual(new Set(['a', 'b']));
  });
});

describe('highlightNodesForSelection', () => {
  const nodes = [
    { id: 'a', position: { x: 0, y: 0 }, data: {} },
    { id: 'b', position: { x: 0, y: 0 }, data: {} },
    { id: 'x', position: { x: 0, y: 0 }, data: {} },
  ];
  const edges = [
    { id: 'e1', source: 'a', target: 'b' },
    { id: 'e2', source: 'x', target: 'y' },
  ];

  it('dims nodes outside component when selected', () => {
    const out = highlightNodesForSelection(nodes, edges, 'a');
    expect(out.find((n) => n.id === 'a')?.data).toMatchObject({ dimmed: false, selected: true });
    expect(out.find((n) => n.id === 'b')?.data).toMatchObject({ dimmed: false, selected: false });
    expect(out.find((n) => n.id === 'x')?.data).toMatchObject({ dimmed: true, selected: false });
  });

  it('clears dimming when selection is null', () => {
    const out = highlightNodesForSelection(nodes, edges, null);
    for (const n of out) {
      expect(n.data).toMatchObject({ dimmed: false, selected: false });
    }
  });
});

describe('highlightEdgesForSelection', () => {
  const edges = [
    { id: 'e1', source: 'a', target: 'b', style: { strokeWidth: 2 } },
    { id: 'e2', source: 'x', target: 'y' },
  ];

  it('fades edges not fully inside component', () => {
    const out = highlightEdgesForSelection(edges, 'a');
    expect(out.find((e) => e.id === 'e1')?.style).toMatchObject({ opacity: 1, strokeWidth: 2 });
    expect(out.find((e) => e.id === 'e2')?.style).toMatchObject({ opacity: 0.08 });
  });

  it('shows all edges when nothing selected', () => {
    const out = highlightEdgesForSelection(edges, null);
    expect(out.every((e) => (e.style as { opacity?: number }).opacity === 1)).toBe(true);
  });
});
