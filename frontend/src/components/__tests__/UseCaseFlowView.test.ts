import { describe, expect, it } from 'vitest';
import type { UseCaseDocument } from '@/api/types';
import { buildUseCaseFlow } from '@/components/useCases/UseCaseFlowView';

const doc: UseCaseDocument = {
  schema: 'marreq.use-case.v1',
  title: 'Export requirements',
  primary_actor: null,
  secondary_actors: [],
  summary: null,
  preconditions: [{ id: 'p_Auth', text: 'Signed in.' }],
  main: [
    { id: 's_Open', text: 'Open the page.' },
    { id: 's_Pick', text: 'Select requirements.' },
    { id: 's_Make', text: 'Generate the file.' },
  ],
  branches: [
    {
      id: 'b_None',
      kind: 'alternative',
      title: 'Nothing selected',
      from_step: 's_Pick',
      steps: [{ id: 's_Ask', text: 'Ask again.' }],
      end: { type: 'rejoin', step: 's_Pick' },
    },
    {
      id: 'b_Fail',
      kind: 'exception',
      title: 'Export fails',
      from_step: 's_Make',
      steps: [],
      end: { type: 'end', outcome: 'failure' },
    },
  ],
  postconditions: [{ id: 'q_File', text: 'The file exists.' }],
};
const labels = { s_Open: '1', s_Pick: '2', s_Make: '3', b_None: 'A2', s_Ask: 'A2.1', b_Fail: 'E3' };

describe('buildUseCaseFlow', () => {
  it('lays the main scenario out top to bottom between start and end', () => {
    const { nodes, edges } = buildUseCaseFlow(doc, labels);
    const main = ['start', 's_Open', 's_Pick', 's_Make', 'end'].map((id) => nodes.find((n) => n.id === id)!);
    expect(main.every((n) => n.position.x === 0)).toBe(true);
    expect(main.map((n) => n.position.y)).toEqual([...main.map((n) => n.position.y)].sort((a, b) => a - b));
    expect(main[0]!.data.text).toBe('Start: Signed in.');
    expect(main[4]!.data.text).toBe('End: The file exists.');
    for (const [from, to] of [['start', 's_Open'], ['s_Open', 's_Pick'], ['s_Pick', 's_Make'], ['s_Make', 'end']]) {
      expect(edges.some((e) => e.source === from && e.target === to)).toBe(true);
    }
  });

  it('puts each branch in a column beside its origin, rejoining or ending', () => {
    const { nodes, edges } = buildUseCaseFlow(doc, labels, 'b_None');
    const origin = nodes.find((n) => n.id === 's_Pick')!;
    const alt = nodes.find((n) => n.id === 'b_None')!;
    expect(alt.position.x).toBeGreaterThan(0);
    expect(alt.position.y).toBeGreaterThan(origin.position.y);
    expect(alt.data.selected).toBe(true);
    expect(nodes.find((n) => n.id === 's_Ask')!.data).toMatchObject({ label: 'A2.1', selected: true });
    const rejoin = edges.find((e) => e.source === 's_Ask' && e.target === 's_Pick')!;
    expect(rejoin).toMatchObject({ label: 'continues at 2', sourceHandle: 'left-out', targetHandle: 'right-in' });

    // One branch column; the exception starts below the alternative's last step.
    const exc = nodes.find((n) => n.id === 'b_Fail')!;
    expect(exc.position.x).toBe(alt.position.x);
    expect(exc.position.y).toBeGreaterThan(nodes.find((n) => n.id === 's_Ask')!.position.y);
    expect(nodes.find((n) => n.id === 'b_Fail:end')!.data.text).toBe('Use case ends without success');
    expect(edges.some((e) => e.source === 'b_Fail' && e.target === 'b_Fail:end')).toBe(true);
  });
});
