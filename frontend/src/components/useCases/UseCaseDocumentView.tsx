import { useEffect, useState, type ReactNode } from 'react';
import type {
  UseCaseBranch,
  UseCaseDocument,
  UseCaseElement,
  UseCaseLabels,
  UseCaseListRef,
  UseCaseOperation,
} from '@/api/types';
import StatementText from '@/components/StatementText';
import { listKey, newElementId } from '@/utils/useCaseDocument';
import BranchCard from './BranchCard';
import ElementRow, { type RowMenuItem } from './ElementRow';

type Props = {
  doc: UseCaseDocument;
  labels: UseCaseLabels;
  editing: boolean;
  /** Send operations to the draft; resolves once the server answered. */
  apply: (ops: UseCaseOperation[]) => Promise<void>;
  selectedBranch: string | null;
  onSelectBranch: (id: string | null) => void;
};

type Pending = { key: string; list: UseCaseListRef; after: string | null; id: string };

function Section({
  title,
  icon,
  extra,
  children,
}: {
  title: string;
  icon: string;
  extra?: ReactNode;
  children: ReactNode;
}) {
  return (
    <section className="rounded-lg border border-stitch-border bg-stitch-surface shadow-sm">
      <div className="flex items-center gap-2 border-b border-stitch-border bg-stitch-elevated/50 px-4 py-2.5">
        <span className="material-symbols-outlined text-lg text-stitch-accent">{icon}</span>
        <h3 className="font-headline text-sm font-bold text-stitch-accent">{title}</h3>
        {extra}
      </div>
      <div className="px-3 py-2">{children}</div>
    </section>
  );
}

function AddButton({ label, onClick }: { label: string; onClick: () => void }) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="mt-1 flex items-center gap-1.5 rounded-md py-1 pl-8 pr-2 text-xs text-stitch-muted hover:text-stitch-accent"
    >
      <span className="material-symbols-outlined text-sm">add</span>
      {label}
    </button>
  );
}

/** The use case document, read-only or as a block editor (screens 2–5 of #388). */
export default function UseCaseDocumentView({
  doc,
  labels,
  editing,
  apply,
  selectedBranch,
  onSelectBranch,
}: Props) {
  const [pending, setPending] = useState<Pending | null>(null);
  const [focusId, setFocusId] = useState<string | null>(null);
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());
  const [drag, setDrag] = useState<{ key: string; id: string } | null>(null);
  const [over, setOver] = useState<{ id: string; before: boolean } | null>(null);

  // Header fields edit locally and save on blur.
  const [title, setTitle] = useState(doc.title);
  const [actor, setActor] = useState(doc.primary_actor ?? '');
  const [secondary, setSecondary] = useState(doc.secondary_actors.join(', '));
  const [summary, setSummary] = useState(doc.summary ?? '');
  useEffect(() => {
    setTitle(doc.title);
    setActor(doc.primary_actor ?? '');
    setSecondary(doc.secondary_actors.join(', '));
    setSummary(doc.summary ?? '');
  }, [doc.title, doc.primary_actor, doc.secondary_actors, doc.summary]);

  const selected = doc.branches.find((b) => b.id === selectedBranch) ?? null;
  const originId = selected?.from_step ?? null;

  function startInsert(list: UseCaseListRef, after: string | null) {
    const prefix = list === 'preconditions' ? 'p' : list === 'postconditions' ? 'q' : 's';
    const id = newElementId(prefix);
    setPending({ key: listKey(list), list, after, id });
    setFocusId(id);
  }

  function rowsFor(list: UseCaseListRef, items: UseCaseElement[]): (UseCaseElement & { pending?: boolean })[] {
    const rows: (UseCaseElement & { pending?: boolean })[] = [...items];
    if (pending && pending.key === listKey(list)) {
      const at = pending.after == null ? 0 : rows.findIndex((r) => r.id === pending.after) + 1;
      rows.splice(at > 0 || pending.after == null ? at : rows.length, 0, {
        id: pending.id,
        text: '',
        pending: true,
      });
    }
    return rows;
  }

  function move(list: UseCaseListRef, items: UseCaseElement[], index: number, delta: -1 | 1) {
    const target = index + delta;
    if (target < 0 || target >= items.length) return;
    const id = items[index]!.id;
    const after = delta < 0 ? (index >= 2 ? items[index - 2]!.id : null) : items[index + 1]!.id;
    setFocusId(id);
    void apply([{ op: 'move_element', id, list, after }]);
  }

  function dropOn(list: UseCaseListRef, items: UseCaseElement[]) {
    if (!drag || !over || drag.key !== listKey(list)) return;
    const without = items.filter((e) => e.id !== drag.id);
    const idx = without.findIndex((e) => e.id === over.id);
    if (idx < 0) return;
    const pos = over.before ? idx : idx + 1;
    const after = pos > 0 ? without[pos - 1]!.id : null;
    const current = items.findIndex((e) => e.id === drag.id);
    const currentAfter = current > 0 ? items[current - 1]!.id : null;
    setDrag(null);
    setOver(null);
    if (after === currentAfter) return;
    void apply([{ op: 'move_element', id: drag.id, list, after }]);
  }

  function addBranch(fromStep: string, kind: UseCaseBranch['kind']) {
    const id = newElementId('b');
    const branch: UseCaseBranch = {
      id,
      kind,
      title: kind === 'alternative' ? 'New alternative' : 'New exception',
      from_step: fromStep,
      steps: [],
      end: kind === 'alternative' ? { type: 'rejoin', step: fromStep } : { type: 'end', outcome: 'failure' },
    };
    void apply([{ op: 'add_branch', branch }]).then(() => {
      onSelectBranch(id);
      startInsert({ branch: id }, null);
    });
  }

  function renderList(
    list: UseCaseListRef,
    items: UseCaseElement[],
    variant: 'step' | 'branch-step' | 'bullet',
    emptyText: string,
  ) {
    const rows = rowsFor(list, items);
    if (rows.length === 0 && !editing) {
      return <p className="py-1.5 pl-1 text-sm italic text-stitch-muted">{emptyText}</p>;
    }
    return rows.map((row) => {
      const index = items.findIndex((e) => e.id === row.id);
      if (row.pending) {
        return (
          <ElementRow
            key={row.id}
            id={row.id}
            label="+"
            text=""
            variant={variant}
            editing
            autoFocus
            onCommit={(text) => {
              setPending(null);
              void apply([{ op: 'add_element', list, after: pendingAfter(), element: { id: row.id, text } }]);
            }}
            onEnter={(text) => {
              setPending(null);
              void apply([{ op: 'add_element', list, after: pendingAfter(), element: { id: row.id, text } }]).then(
                () => startInsert(list, row.id),
              );
            }}
            onCancelEmpty={() => setPending(null)}
          />
        );
      }
      const isMain = list === 'main';
      const menu: RowMenuItem[] = [];
      if (isMain) {
        menu.push(
          { icon: 'alt_route', label: 'Add alternative from this step', onSelect: () => addBranch(row.id, 'alternative') },
          { icon: 'report', label: 'Add exception from this step', onSelect: () => addBranch(row.id, 'exception') },
        );
      }
      menu.push(
        {
          icon: 'arrow_upward',
          label: 'Move up',
          disabled: index <= 0,
          onSelect: () => move(list, items, index, -1),
        },
        {
          icon: 'arrow_downward',
          label: 'Move down',
          disabled: index >= items.length - 1,
          onSelect: () => move(list, items, index, 1),
        },
        {
          icon: 'delete',
          label: variant === 'bullet' ? 'Delete' : 'Delete step',
          danger: true,
          onSelect: () => void apply([{ op: 'remove_element', id: row.id }]),
        },
      );
      return (
        <ElementRow
          key={row.id}
          id={row.id}
          label={labels[row.id]}
          text={row.text}
          variant={variant}
          editing={editing}
          autoFocus={focusId === row.id}
          highlighted={row.id === originId}
          onCommit={(text) => void apply([{ op: 'update_text', id: row.id, text }])}
          onEnter={(text) => {
            const save = text !== row.text ? apply([{ op: 'update_text', id: row.id, text }]) : Promise.resolve();
            void save.then(() => startInsert(list, row.id));
          }}
          onMove={(delta) => move(list, items, index, delta)}
          onInsertBelow={() => startInsert(list, row.id)}
          menu={menu}
          dragHandlers={
            editing
              ? {
                  onDragStart: () => setDrag({ key: listKey(list), id: row.id }),
                  onDragOver: (before) => {
                    if (drag?.key === listKey(list)) setOver({ id: row.id, before });
                  },
                  onDrop: () => dropOn(list, items),
                  onDragEnd: () => {
                    setDrag(null);
                    setOver(null);
                  },
                  dropIndicator:
                    over?.id === row.id && drag?.key === listKey(list) && drag.id !== row.id
                      ? over.before
                        ? 'before'
                        : 'after'
                      : null,
                }
              : undefined
          }
        />
      );
    });
  }

  function pendingAfter(): string | null {
    return pending?.after ?? null;
  }

  function saveHeader() {
    const op: UseCaseOperation = { op: 'set_header' };
    let changed = false;
    const t = title.trim();
    if (t && t !== doc.title) {
      op.title = t;
      changed = true;
    } else if (!t) {
      setTitle(doc.title);
    }
    const a = actor.trim();
    if (a !== (doc.primary_actor ?? '')) {
      op.primary_actor = a || null;
      changed = true;
    }
    const s = secondary
      .split(',')
      .map((x) => x.trim())
      .filter(Boolean);
    if (s.join('\u0000') !== doc.secondary_actors.join('\u0000')) {
      op.secondary_actors = s;
      changed = true;
    }
    const sum = summary.trim();
    if (sum !== (doc.summary ?? '')) {
      op.summary = sum || null;
      changed = true;
    }
    if (changed) void apply([op]);
  }

  const inputClass =
    'w-full rounded-md border border-stitch-border bg-stitch-elevated px-3 py-2 text-sm text-stitch-fg';

  return (
    <div className="min-w-0 flex-1 space-y-3">
      {editing ? (
        <section className="grid grid-cols-1 gap-3 rounded-lg border border-stitch-border bg-stitch-surface p-4 shadow-sm md:grid-cols-2">
          <label className="block md:col-span-2">
            <span className="mb-1 block text-[10px] font-bold uppercase text-stitch-muted">Title</span>
            <input value={title} onChange={(e) => setTitle(e.target.value)} onBlur={saveHeader} className={inputClass} />
          </label>
          <label className="block">
            <span className="mb-1 block text-[10px] font-bold uppercase text-stitch-muted">Primary actor</span>
            <input value={actor} onChange={(e) => setActor(e.target.value)} onBlur={saveHeader} className={inputClass} />
          </label>
          <label className="block">
            <span className="mb-1 block text-[10px] font-bold uppercase text-stitch-muted">
              Secondary actors (comma-separated)
            </span>
            <input
              value={secondary}
              onChange={(e) => setSecondary(e.target.value)}
              onBlur={saveHeader}
              className={inputClass}
            />
          </label>
          <label className="block md:col-span-2">
            <span className="mb-1 block text-[10px] font-bold uppercase text-stitch-muted">Summary</span>
            <textarea
              value={summary}
              rows={2}
              onChange={(e) => setSummary(e.target.value)}
              onBlur={saveHeader}
              className={inputClass}
            />
          </label>
        </section>
      ) : doc.summary ? (
        <div className="rounded-lg border border-stitch-border bg-stitch-surface px-4 py-3 text-sm text-stitch-fg shadow-sm">
          <StatementText source={doc.summary} />
        </div>
      ) : null}

      <Section title="Preconditions" icon="login">
        {renderList('preconditions', doc.preconditions, 'bullet', 'None.')}
        {editing ? <AddButton label="Add precondition" onClick={() => startInsert('preconditions', doc.preconditions.at(-1)?.id ?? null)} /> : null}
      </Section>

      <Section
        title="Main scenario"
        icon="format_list_numbered"
        extra={
          <span className="ml-auto text-xs text-stitch-muted">
            {doc.main.length} step{doc.main.length === 1 ? '' : 's'}
          </span>
        }
      >
        {renderList('main', doc.main, 'step', 'No steps yet.')}
        {editing ? (
          <AddButton label="Add step · or press Enter at the end of a step" onClick={() => startInsert('main', doc.main.at(-1)?.id ?? null)} />
        ) : null}
      </Section>

      {doc.branches.length > 0 ? (
        <div className="space-y-2">
          <div className="flex items-center gap-2">
            <p className="text-[11px] font-bold uppercase tracking-[0.14em] text-stitch-muted">
              Alternatives &amp; exceptions ({doc.branches.length})
            </p>
            <button
              type="button"
              onClick={() =>
                setCollapsed(collapsed.size > 0 ? new Set() : new Set(doc.branches.map((b) => b.id)))
              }
              className="ml-auto text-xs font-semibold text-stitch-accent hover:underline"
            >
              {collapsed.size > 0 ? 'Expand all' : 'Collapse all'}
            </button>
          </div>
          {doc.branches.map((b) => (
            <BranchCard
              key={b.id}
              branch={b}
              labels={labels}
              main={doc.main}
              editing={editing}
              open={!collapsed.has(b.id)}
              selected={b.id === selectedBranch}
              onToggle={() => {
                const next = new Set(collapsed);
                if (next.has(b.id)) next.delete(b.id);
                else next.add(b.id);
                setCollapsed(next);
              }}
              onSelect={() => onSelectBranch(b.id === selectedBranch ? null : b.id)}
              onUpdate={(change) => void apply([{ op: 'update_branch', id: b.id, ...change }])}
              onRemove={() => void apply([{ op: 'remove_branch', id: b.id }])}
            >
              {renderList({ branch: b.id }, b.steps, 'branch-step', 'No steps.')}
              {editing ? (
                <AddButton label="Add step" onClick={() => startInsert({ branch: b.id }, b.steps.at(-1)?.id ?? null)} />
              ) : null}
            </BranchCard>
          ))}
        </div>
      ) : null}

      <Section title="Postconditions" icon="task_alt">
        {renderList('postconditions', doc.postconditions, 'bullet', 'None.')}
        {editing ? (
          <AddButton label="Add postcondition" onClick={() => startInsert('postconditions', doc.postconditions.at(-1)?.id ?? null)} />
        ) : null}
      </Section>
    </div>
  );
}
