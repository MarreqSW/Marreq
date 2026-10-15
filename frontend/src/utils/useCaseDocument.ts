import type {
  UseCaseBranch,
  UseCaseDocument,
  UseCaseElement,
  UseCaseListRef,
  UseCaseOperation,
} from '@/api/types';

/** Use case document helpers for the editor (issue #399). The server is the authority on labels and validity. */

const ALPHABET = '0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz';

/** A new element id such as `s_k3Q9xT0a`. Ids are chosen by the client so undo can refer to them. */
export function newElementId(prefix: 'p' | 'q' | 's' | 'b'): string {
  const bytes = new Uint8Array(10);
  crypto.getRandomValues(bytes);
  let out = `${prefix}_`;
  for (const b of bytes) out += ALPHABET[b % ALPHABET.length];
  return out;
}

export function listKey(list: UseCaseListRef): string {
  return typeof list === 'string' ? list : `branch:${list.branch}`;
}

export function getList(doc: UseCaseDocument, list: UseCaseListRef): UseCaseElement[] {
  if (typeof list !== 'string') return doc.branches.find((b) => b.id === list.branch)?.steps ?? [];
  return doc[list];
}

/** Where an element is: its list and the id of the element before it (`null` when first). */
export function locate(
  doc: UseCaseDocument,
  id: string,
): { list: UseCaseListRef; index: number; after: string | null; element: UseCaseElement } | null {
  const lists: UseCaseListRef[] = [
    'preconditions',
    'main',
    'postconditions',
    ...doc.branches.map((b) => ({ branch: b.id })),
  ];
  for (const list of lists) {
    const items = getList(doc, list);
    const index = items.findIndex((e) => e.id === id);
    if (index >= 0) {
      return { list, index, after: index > 0 ? items[index - 1]!.id : null, element: items[index]! };
    }
  }
  return null;
}

/** Branches that start from, or rejoin at, a main step. */
export function branchesDependingOn(doc: UseCaseDocument, stepId: string): UseCaseBranch[] {
  return doc.branches.filter(
    (b) => b.from_step === stepId || (b.end.type === 'rejoin' && b.end.step === stepId),
  );
}

/**
 * The operations that undo `ops` when applied to the document that results
 * from them. `doc` is the document before `ops`. Returns `null` when an
 * operation cannot be undone from here.
 */
export function inverseOperations(
  doc: UseCaseDocument,
  ops: UseCaseOperation[],
): UseCaseOperation[] | null {
  // Operations are inverted one at a time against the document they apply
  // to; the editor sends single-operation batches, so `doc` is that document.
  if (ops.length !== 1) return null;
  const op = ops[0]!;
  switch (op.op) {
    case 'set_header': {
      const inv: UseCaseOperation = { op: 'set_header' };
      if (op.title !== undefined) inv.title = doc.title;
      if (op.primary_actor !== undefined) inv.primary_actor = doc.primary_actor;
      if (op.secondary_actors !== undefined) inv.secondary_actors = doc.secondary_actors;
      if (op.summary !== undefined) inv.summary = doc.summary;
      return [inv];
    }
    case 'add_element':
      return [{ op: 'remove_element', id: op.element.id }];
    case 'update_text': {
      const at = locate(doc, op.id);
      return at ? [{ op: 'update_text', id: op.id, text: at.element.text }] : null;
    }
    case 'move_element': {
      const at = locate(doc, op.id);
      return at ? [{ op: 'move_element', id: op.id, list: at.list, after: at.after }] : null;
    }
    case 'remove_element': {
      const at = locate(doc, op.id);
      if (!at) return null;
      const restore: UseCaseOperation[] = [
        { op: 'add_element', list: at.list, after: at.after, element: at.element },
      ];
      const dependents = branchesDependingOn(doc, op.id);
      if (dependents.length === 0) return restore;
      if (op.dependents === 'cascade') {
        return [...restore, ...dependents.map((b): UseCaseOperation => ({ op: 'add_branch', branch: b }))];
      }
      return [
        ...restore,
        ...dependents.map(
          (b): UseCaseOperation => ({ op: 'update_branch', id: b.id, from_step: b.from_step, end: b.end }),
        ),
      ];
    }
    case 'add_branch':
      return [{ op: 'remove_branch', id: op.branch.id }];
    case 'update_branch': {
      const b = doc.branches.find((x) => x.id === op.id);
      if (!b) return null;
      const inv: UseCaseOperation = { op: 'update_branch', id: op.id };
      if (op.title !== undefined) inv.title = b.title;
      if (op.kind !== undefined) inv.kind = b.kind;
      if (op.from_step !== undefined) inv.from_step = b.from_step;
      if (op.end !== undefined) inv.end = b.end;
      return [inv];
    }
    case 'remove_branch': {
      const b = doc.branches.find((x) => x.id === op.id);
      return b ? [{ op: 'add_branch', branch: b }] : null;
    }
  }
}

/** The approval chip for a version's state. */
export function approvalChip(state: string | null | undefined): { label: string; className: string } {
  switch ((state ?? '').toLowerCase()) {
    case 'approved':
      return { label: 'Approved', className: 'bg-emerald-500/15 text-emerald-700 dark:text-emerald-300' };
    case 'reviewed':
      return { label: 'Reviewed', className: 'bg-sky-500/15 text-sky-700 dark:text-sky-300' };
    case 'draft':
      return { label: 'Draft', className: 'bg-slate-500/15 text-slate-700 dark:text-slate-300' };
    default:
      return { label: 'Unpublished', className: 'bg-amber-500/15 text-amber-800 dark:text-amber-300' };
  }
}

/** Server timestamps are UTC without a zone; read them as UTC. */
export function parseServerTime(iso: string): Date {
  return new Date(/[zZ]|[+-]\d\d:?\d\d$/.test(iso) ? iso : `${iso}Z`);
}

export function formatRelative(iso: string, now: Date = new Date()): string {
  const d = parseServerTime(iso);
  const ms = now.getTime() - d.getTime();
  if (Number.isNaN(d.getTime())) return '';
  const minutes = Math.floor(ms / 60000);
  if (minutes < 1) return 'just now';
  if (minutes < 60) return `${minutes} min ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours} h ago`;
  const days = Math.floor(hours / 24);
  if (days < 7) return `${days} d ago`;
  return d.toLocaleDateString(undefined, { dateStyle: 'medium' });
}
