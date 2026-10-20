import { describe, expect, it } from 'vitest';
import type { UseCaseDocument } from '@/api/types';
import {
  branchesDependingOn,
  formatRelative,
  inverseOperations,
  locate,
  newElementId,
  parseServerTime,
} from '../useCaseDocument';

const doc: UseCaseDocument = {
  schema: 'marreq.use-case.v1',
  title: 'Export requirements',
  primary_actor: 'Engineer',
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
  ],
  postconditions: [],
};

describe('useCaseDocument', () => {
  it('generates ids of the right kind', () => {
    expect(newElementId('s')).toMatch(/^s_[A-Za-z0-9]{10}$/);
    expect(newElementId('b')).not.toEqual(newElementId('b'));
  });

  it('locates elements in every list', () => {
    expect(locate(doc, 's_Pick')).toMatchObject({ list: 'main', index: 1, after: 's_Open' });
    expect(locate(doc, 's_Ask')).toMatchObject({ list: { branch: 'b_None' }, after: null });
    expect(locate(doc, 'p_Auth')?.list).toBe('preconditions');
    expect(locate(doc, 's_Nope')).toBeNull();
    expect(branchesDependingOn(doc, 's_Pick').map((b) => b.id)).toEqual(['b_None']);
  });

  it('inverts the editor operations', () => {
    expect(inverseOperations(doc, [{ op: 'update_text', id: 's_Make', text: 'x' }])).toEqual([
      { op: 'update_text', id: 's_Make', text: 'Generate the file.' },
    ]);
    expect(inverseOperations(doc, [{ op: 'move_element', id: 's_Make', after: null }])).toEqual([
      { op: 'move_element', id: 's_Make', list: 'main', after: 's_Pick' },
    ]);
    expect(
      inverseOperations(doc, [{ op: 'add_element', list: 'main', element: { id: 's_New', text: 'y' } }]),
    ).toEqual([{ op: 'remove_element', id: 's_New' }]);
    expect(inverseOperations(doc, [{ op: 'set_header', title: 'Other', summary: 'Why' }])).toEqual([
      { op: 'set_header', title: 'Export requirements', summary: null },
    ]);
    expect(inverseOperations(doc, [{ op: 'remove_branch', id: 'b_None' }])).toEqual([
      { op: 'add_branch', branch: doc.branches[0] },
    ]);
    expect(inverseOperations(doc, [{ op: 'update_branch', id: 'b_None', from_step: 's_Open' }])).toEqual([
      { op: 'update_branch', id: 'b_None', from_step: 's_Pick' },
    ]);
  });

  it('restores a removed step with what depended on it', () => {
    const restore = { op: 'add_element', list: 'main', after: 's_Open', element: doc.main[1] };
    expect(inverseOperations(doc, [{ op: 'remove_element', id: 's_Pick', dependents: 'cascade' }])).toEqual([
      restore,
      { op: 'add_branch', branch: doc.branches[0] },
    ]);
    expect(
      inverseOperations(doc, [{ op: 'remove_element', id: 's_Pick', dependents: { rebranch_to: 's_Open' } }]),
    ).toEqual([
      restore,
      { op: 'update_branch', id: 'b_None', from_step: 's_Pick', end: { type: 'rejoin', step: 's_Pick' } },
    ]);
    expect(inverseOperations(doc, [{ op: 'update_text', id: 's_Nope', text: '' }])).toBeNull();
  });

  it('reads server timestamps as UTC', () => {
    expect(parseServerTime('2026-10-15T08:00:00').toISOString()).toBe('2026-10-15T08:00:00.000Z');
    expect(parseServerTime('2026-10-15T08:00:00+02:00').toISOString()).toBe('2026-10-15T06:00:00.000Z');
    const now = new Date('2026-10-15T10:00:00Z');
    expect(formatRelative('2026-10-15T08:00:00', now)).toBe('2 h ago');
    expect(formatRelative('2026-10-15T09:59:50', now)).toBe('just now');
  });
});
