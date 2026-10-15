import { useEffect, useState, type ReactNode } from 'react';
import type { UseCaseBranch, UseCaseBranchEnd, UseCaseElement, UseCaseLabels } from '@/api/types';

type Props = {
  branch: UseCaseBranch;
  labels: UseCaseLabels;
  main: UseCaseElement[];
  editing: boolean;
  open: boolean;
  selected: boolean;
  onToggle: () => void;
  onSelect: () => void;
  onUpdate?: (change: {
    title?: string;
    kind?: UseCaseBranch['kind'];
    from_step?: string;
    end?: UseCaseBranchEnd;
  }) => void;
  onRemove?: () => void;
  /** The branch's steps, rendered by the editor. */
  children: ReactNode;
};

function endValue(end: UseCaseBranchEnd): string {
  return end.type === 'rejoin' ? `rejoin:${end.step}` : `end:${end.outcome}`;
}

function parseEnd(value: string): UseCaseBranchEnd {
  const [type, rest] = value.split(':') as [string, string];
  return type === 'rejoin'
    ? { type: 'rejoin', step: rest }
    : { type: 'end', outcome: rest === 'success' ? 'success' : 'failure' };
}

/** An alternative or exception, branching from a main step (screens 2 and 5 of #388). */
export default function BranchCard({
  branch,
  labels,
  main,
  editing,
  open,
  selected,
  onToggle,
  onSelect,
  onUpdate,
  onRemove,
  children,
}: Props) {
  const alt = branch.kind === 'alternative';
  const [title, setTitle] = useState(branch.title);
  useEffect(() => setTitle(branch.title), [branch.title]);
  const origin = labels[branch.from_step] ?? '?';
  const endText =
    branch.end.type === 'rejoin'
      ? `Continues at step ${labels[branch.end.step] ?? '?'}`
      : branch.end.outcome === 'success'
        ? 'Use case ends (success)'
        : 'Use case ends without success';

  return (
    <div
      id={`branch-${branch.id}`}
      className={`rounded-md border border-l-4 border-stitch-border bg-stitch-surface ${
        alt ? 'border-l-sky-600' : 'border-l-rose-600'
      } ${selected ? 'ring-2 ring-amber-500' : ''}`}
    >
      <div className="flex flex-wrap items-center gap-2 px-3 py-2">
        <button
          type="button"
          onClick={onToggle}
          aria-expanded={open}
          aria-label={open ? 'Collapse' : 'Expand'}
          className="rounded-sm text-stitch-muted hover:text-stitch-fg"
        >
          <span className="material-symbols-outlined text-base">{open ? 'expand_more' : 'chevron_right'}</span>
        </button>
        <button type="button" onClick={onSelect} className="font-mono text-xs font-bold text-stitch-fg">
          {labels[branch.id] ?? ''}
        </button>
        {editing && onUpdate ? (
          <select
            value={branch.kind}
            aria-label="Branch kind"
            onChange={(e) => onUpdate({ kind: e.target.value as UseCaseBranch['kind'] })}
            className="rounded-sm border border-stitch-border bg-stitch-elevated px-1 py-0.5 text-[10px] font-bold uppercase tracking-wider text-stitch-fg"
          >
            <option value="alternative">Alternative</option>
            <option value="exception">Exception</option>
          </select>
        ) : (
          <span
            className={`rounded-sm px-1.5 py-0.5 text-[10px] font-bold uppercase tracking-wider ${
              alt
                ? 'bg-sky-500/15 text-sky-800 dark:text-sky-300'
                : 'bg-rose-500/15 text-rose-800 dark:text-rose-300'
            }`}
          >
            {alt ? 'Alternative' : 'Exception'}
          </span>
        )}
        {editing && onUpdate ? (
          <input
            value={title}
            aria-label="Branch title"
            onChange={(e) => setTitle(e.target.value)}
            onBlur={() => {
              const next = title.trim();
              if (next && next !== branch.title) onUpdate({ title: next });
              else setTitle(branch.title);
            }}
            onKeyDown={(e) => {
              if (e.key === 'Enter') e.currentTarget.blur();
            }}
            className="min-w-40 flex-1 rounded-sm border border-transparent bg-transparent px-1 text-sm font-semibold text-stitch-fg hover:border-stitch-border focus:border-stitch-accent focus:outline-hidden"
          />
        ) : (
          <button type="button" onClick={onSelect} className="text-left text-sm font-semibold text-stitch-fg">
            {branch.title}
          </button>
        )}
        <span className="ml-auto flex items-center gap-1 whitespace-nowrap text-xs text-stitch-muted">
          <span className="material-symbols-outlined text-sm">subdirectory_arrow_right</span>
          {editing && onUpdate ? (
            <label className="flex items-center gap-1">
              from step
              <select
                value={branch.from_step}
                aria-label="Branches from step"
                onChange={(e) => onUpdate({ from_step: e.target.value })}
                className="rounded-sm border border-stitch-border bg-stitch-elevated px-1 py-0.5 text-xs text-stitch-fg"
              >
                {main.map((s) => (
                  <option key={s.id} value={s.id}>
                    {labels[s.id]}
                  </option>
                ))}
              </select>
            </label>
          ) : (
            <span>
              branches from <strong className="text-stitch-fg">step {origin}</strong>
            </span>
          )}
          {editing && onRemove ? (
            <button
              type="button"
              title="Delete branch"
              aria-label={`Delete ${labels[branch.id] ?? 'branch'}`}
              onClick={onRemove}
              className="ml-1 rounded-sm p-0.5 text-stitch-muted hover:bg-stitch-elevated hover:text-stitch-danger"
            >
              <span className="material-symbols-outlined text-base">delete</span>
            </button>
          ) : null}
        </span>
      </div>
      {open ? (
        <div className="border-t border-stitch-border/60 px-3 pb-2 pt-1.5 pl-8">
          {children}
          <div className="mt-1 flex items-center gap-1.5 pl-1 text-xs text-stitch-muted">
            <span className="material-symbols-outlined text-sm">
              {branch.end.type === 'rejoin' ? 'undo' : 'block'}
            </span>
            {editing && onUpdate ? (
              <select
                value={endValue(branch.end)}
                aria-label="Where the branch ends"
                onChange={(e) => onUpdate({ end: parseEnd(e.target.value) })}
                className="rounded-sm border border-stitch-border bg-stitch-elevated px-1 py-0.5 text-xs text-stitch-fg"
              >
                {main.map((s) => (
                  <option key={s.id} value={`rejoin:${s.id}`}>
                    Continues at step {labels[s.id]}
                  </option>
                ))}
                <option value="end:success">Use case ends (success)</option>
                <option value="end:failure">Use case ends without success</option>
              </select>
            ) : (
              <span>{endText}</span>
            )}
          </div>
        </div>
      ) : null}
    </div>
  );
}
