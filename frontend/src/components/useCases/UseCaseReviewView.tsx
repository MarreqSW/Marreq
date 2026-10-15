import { useEffect, useMemo, useState } from 'react';
import { getUseCaseDraftDiff, getUseCaseVersionDiff } from '@/api/client';
import type {
  UseCaseChange,
  UseCaseComment,
  UseCaseDiff,
  UseCaseElementDiff,
  UseCaseVersionSummary,
} from '@/api/types';

const CHANGE_STYLE: Record<UseCaseChange, [string, string]> = {
  added: ['Added', 'bg-emerald-500/15 text-emerald-800 dark:text-emerald-300'],
  removed: ['Removed', 'bg-red-500/15 text-red-800 dark:text-red-300'],
  edited: ['Edited', 'bg-amber-500/15 text-amber-800 dark:text-amber-300'],
  moved: ['Moved', 'bg-violet-500/15 text-violet-800 dark:text-violet-300'],
  rebranched: ['Rebranched', 'bg-sky-500/15 text-sky-800 dark:text-sky-300'],
};

const SECTION: Record<string, string> = {
  preconditions: 'Preconditions',
  main: 'Main scenario',
  branch: 'Alternatives & exceptions',
  branches: 'Alternatives & exceptions',
  postconditions: 'Postconditions',
};

type Props = {
  projectId: number;
  useCaseId: number;
  versions: UseCaseVersionSummary[];
  hasDraft: boolean;
  comments: UseCaseComment[];
  onCommentElement: (elementId: string) => void;
  /** Version to compare from (a baseline's); the current version is the other side. */
  initialFrom?: number;
};

/** `draft` or a version id. */
type End = 'draft' | number;

function endLabel(end: End, versions: UseCaseVersionSummary[]): string {
  if (end === 'draft') return 'Draft';
  const v = versions.find((x) => x.id === end);
  return v ? `v${v.version_number}` : '?';
}

function label(e: UseCaseElementDiff): string {
  const old = e.old?.label;
  const now = e.new?.label;
  if (old && now && old !== now) return `${old} → ${now}`;
  return now ?? old ?? '';
}

function ElementText({ e }: { e: UseCaseElementDiff }) {
  if (e.text_diff) {
    return (
      <span>
        {e.text_diff.map((c, i) =>
          c.op === 'equal' ? (
            <span key={i}>{c.text}</span>
          ) : c.op === 'delete' ? (
            <del key={i} className="rounded-sm bg-red-500/15 text-red-800 dark:text-red-300">
              {c.text}
            </del>
          ) : (
            <ins key={i} className="rounded-sm bg-emerald-500/15 text-emerald-800 no-underline dark:text-emerald-300">
              {c.text}
            </ins>
          ),
        )}
      </span>
    );
  }
  const side = e.new ?? e.old;
  return <span className={e.changes.includes('removed') ? 'line-through opacity-70' : ''}>{side?.text}</span>;
}

/** Structural review of a version or the draft against another (screen 8 of #388, issue #401). */
export default function UseCaseReviewView({
  projectId,
  useCaseId,
  versions,
  hasDraft,
  comments,
  onCommentElement,
  initialFrom,
}: Props) {
  const [to, setTo] = useState<End>(
    hasDraft && initialFrom == null ? 'draft' : (versions[0]?.id ?? 'draft'),
  );
  const [from, setFrom] = useState<number | null>(
    initialFrom ?? (hasDraft ? (versions[0]?.id ?? null) : (versions[1]?.id ?? null)),
  );
  const [showUnchanged, setShowUnchanged] = useState(false);
  const [diff, setDiff] = useState<UseCaseDiff | null>(null);
  const [err, setErr] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    setErr(null);
    const load =
      to === 'draft'
        ? getUseCaseDraftDiff(projectId, useCaseId, showUnchanged)
        : from != null
          ? getUseCaseVersionDiff(projectId, useCaseId, from, to, showUnchanged)
          : null;
    if (!load) {
      setDiff(null);
      return;
    }
    load
      .then((d) => live && setDiff(d))
      .catch((e: unknown) => live && setErr(e instanceof Error ? e.message : 'Failed to load the comparison'));
    return () => {
      live = false;
    };
  }, [projectId, useCaseId, from, to, showUnchanged]);

  const commentCount = useMemo(() => {
    const m = new Map<string, number>();
    for (const c of comments) if (c.element_id) m.set(c.element_id, (m.get(c.element_id) ?? 0) + 1);
    return m;
  }, [comments]);

  const groups = useMemo(() => {
    const out: [string, UseCaseElementDiff[]][] = [];
    for (const e of diff?.elements ?? []) {
      const section = e.changes.includes('removed')
        ? 'Removed'
        : (SECTION[(e.new ?? e.old)?.list ?? 'main'] ?? 'Main scenario');
      const last = out.at(-1);
      if (last && last[0] === section) last[1].push(e);
      else out.push([section, [e]]);
    }
    return out;
  }, [diff]);

  const select = 'rounded-md border border-stitch-border bg-stitch-elevated px-2 py-1 text-xs text-stitch-fg';
  const totalChanges = diff ? Object.values(diff.summary).reduce((a, b) => a + b, 0) : 0;

  return (
    <div className="min-w-0 flex-1 space-y-3">
      <div className="flex flex-wrap items-center gap-2 rounded-lg border border-stitch-border bg-stitch-surface px-4 py-2.5 text-xs shadow-sm">
        <span className="material-symbols-outlined text-lg text-stitch-accent">compare</span>
        <label className="flex items-center gap-1 text-stitch-muted">
          Compare
          <select
            aria-label="Compare from"
            value={from ?? ''}
            onChange={(e) => setFrom(e.target.value ? Number(e.target.value) : null)}
            className={select}
          >
            {versions.map((v) => (
              <option key={v.id} value={v.id}>
                v{v.version_number}
              </option>
            ))}
          </select>
        </label>
        <label className="flex items-center gap-1 text-stitch-muted">
          with
          <select
            aria-label="Compare to"
            value={String(to)}
            onChange={(e) => setTo(e.target.value === 'draft' ? 'draft' : Number(e.target.value))}
            className={select}
          >
            {hasDraft ? <option value="draft">Draft</option> : null}
            {versions.map((v) => (
              <option key={v.id} value={v.id}>
                v{v.version_number}
              </option>
            ))}
          </select>
        </label>
        <label className="ml-auto flex items-center gap-1.5 text-stitch-muted">
          <input type="checkbox" checked={showUnchanged} onChange={(e) => setShowUnchanged(e.target.checked)} />
          Show unchanged
        </label>
      </div>

      {err ? (
        <div role="alert" className="rounded-lg border border-red-500/40 bg-red-500/10 px-4 py-2 text-sm text-red-800 dark:text-red-200">
          {err}
        </div>
      ) : null}
      {to !== 'draft' && from == null ? (
        <p className="rounded-lg border border-dashed border-stitch-border p-6 text-center text-sm text-stitch-muted">
          Only one version: nothing to compare yet.
        </p>
      ) : null}

      {diff ? (
        <>
          <div className="flex flex-wrap items-center gap-2 text-xs" aria-label="Summary">
            <span className="font-semibold text-stitch-fg">
              {diff.from ? endLabel(diff.from.version_id ?? 'draft', versions) : 'Nothing'} → {endLabel(to, versions)}:
            </span>
            {totalChanges === 0 && Object.keys(diff.header).length === 0 ? (
              <span className="text-stitch-muted">no changes</span>
            ) : (
              (Object.keys(CHANGE_STYLE) as UseCaseChange[])
                .filter((k) => diff.summary[k] > 0)
                .map((k) => (
                  <span key={k} className={`rounded-sm px-1.5 py-0.5 font-bold uppercase tracking-wider ${CHANGE_STYLE[k][1]}`}>
                    {diff.summary[k]} {CHANGE_STYLE[k][0]}
                  </span>
                ))
            )}
          </div>

          {Object.entries(diff.header).length > 0 ? (
            <section className="rounded-lg border border-stitch-border bg-stitch-surface px-4 py-3 text-sm shadow-sm">
              <h3 className="mb-1 text-[10px] font-bold uppercase tracking-[0.12em] text-stitch-muted">Header</h3>
              {Object.entries(diff.header).map(([field, change]) => (
                <p key={field} className="text-stitch-fg">
                  <span className="text-stitch-muted">{field.replace('_', ' ')}: </span>
                  <del className="text-red-700 dark:text-red-300">{JSON.stringify(change?.old)}</del>{' '}
                  → <ins className="text-emerald-700 no-underline dark:text-emerald-300">{JSON.stringify(change?.new)}</ins>
                </p>
              ))}
            </section>
          ) : null}

          {groups.map(([section, rows]) => (
            <section key={section} className="rounded-lg border border-stitch-border bg-stitch-surface shadow-sm">
              <h3 className="border-b border-stitch-border bg-stitch-elevated/50 px-4 py-2 font-headline text-sm font-bold text-stitch-accent">
                {section}
              </h3>
              <ul className="divide-y divide-stitch-border/60">
                {rows.map((e) => (
                  <li key={e.id} data-element-id={e.id} className="flex items-start gap-3 px-4 py-2 text-sm">
                    <span className="mt-0.5 w-16 shrink-0 font-mono text-[11px] font-bold text-stitch-muted">{label(e)}</span>
                    <div className="min-w-0 flex-1 text-stitch-fg">
                      {e.kind === 'branch' ? (
                        <span className="mr-1 text-[10px] font-bold uppercase text-stitch-muted">
                          {(e.new ?? e.old)?.kind}
                        </span>
                      ) : null}
                      <ElementText e={e} />
                      {e.changes.includes('rebranched') ? (
                        <p className="mt-0.5 text-xs text-sky-700 dark:text-sky-300">
                          Now branches from a different step or ends differently.
                        </p>
                      ) : null}
                    </div>
                    <span className="flex shrink-0 flex-wrap items-center justify-end gap-1">
                      {e.changes.map((c) => (
                        <span key={c} className={`rounded-sm px-1.5 py-0.5 text-[10px] font-bold uppercase tracking-wider ${CHANGE_STYLE[c][1]}`}>
                          {CHANGE_STYLE[c][0]}
                        </span>
                      ))}
                      {e.kind !== 'branch' && !e.changes.includes('removed') ? (
                        <button
                          type="button"
                          onClick={() => onCommentElement(e.id)}
                          aria-label={`Comment on ${label(e)}`}
                          className="inline-flex items-center gap-0.5 rounded-sm px-1 text-xs text-stitch-muted hover:text-stitch-accent"
                        >
                          <span className="material-symbols-outlined text-sm">add_comment</span>
                          {commentCount.get(e.id) ?? ''}
                        </button>
                      ) : null}
                    </span>
                  </li>
                ))}
              </ul>
            </section>
          ))}
        </>
      ) : null}
    </div>
  );
}
