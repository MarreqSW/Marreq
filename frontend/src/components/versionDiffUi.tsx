import type { CustomFieldDiff, SingleValueDiff, TextDiffResult } from '@/api/types';

function meaningful(lines: string[]): string[] {
  return lines.filter((line) => line.length > 0);
}

export function TextDiffField({ label, diff }: { label: string; diff: TextDiffResult }) {
  const removed = meaningful(diff.removed);
  const added = meaningful(diff.added);
  const unchanged = meaningful(diff.unchanged);
  const changed = removed.length > 0 || added.length > 0;

  return (
    <section className="rounded-xl border border-stitch-border bg-stitch-canvas/60 p-4">
      <div className="mb-3 flex items-center justify-between gap-3">
        <h3 className="text-xs font-bold uppercase tracking-widest text-stitch-fg">{label}</h3>
        <span
          className={`rounded px-2 py-0.5 text-[10px] font-bold uppercase tracking-wider ${
            changed
              ? 'bg-amber-500/15 text-amber-800 dark:text-amber-200'
              : 'bg-stitch-elevated text-stitch-muted'
          }`}
        >
          {changed ? 'Changed' : 'Unchanged'}
        </span>
      </div>
      {removed.length > 0 ? (
        <div className="mb-3">
          <p className="mb-1 text-[10px] font-bold uppercase tracking-wider text-red-700 dark:text-red-300">
            Removed
          </p>
          <pre className="whitespace-pre-wrap rounded-lg border border-red-500/25 bg-red-500/10 p-3 font-mono text-xs leading-relaxed text-red-900 dark:text-red-100">
            {removed.join('\n')}
          </pre>
        </div>
      ) : null}
      {added.length > 0 ? (
        <div className="mb-3">
          <p className="mb-1 text-[10px] font-bold uppercase tracking-wider text-emerald-700 dark:text-emerald-300">
            Added
          </p>
          <pre className="whitespace-pre-wrap rounded-lg border border-emerald-500/25 bg-emerald-500/10 p-3 font-mono text-xs leading-relaxed text-emerald-900 dark:text-emerald-100">
            {added.join('\n')}
          </pre>
        </div>
      ) : null}
      {unchanged.length > 0 ? (
        <details className="text-xs text-stitch-muted" open={!changed}>
          <summary className="cursor-pointer font-semibold">
            Unchanged ({unchanged.length} {unchanged.length === 1 ? 'line' : 'lines'})
          </summary>
          <pre className="mt-2 whitespace-pre-wrap rounded-lg bg-stitch-elevated p-3 font-mono text-xs leading-relaxed text-stitch-muted">
            {unchanged.join('\n')}
          </pre>
        </details>
      ) : !changed ? (
        <p className="text-xs text-stitch-muted">No content in either version.</p>
      ) : null}
    </section>
  );
}

function valueLabel(diff: SingleValueDiff, side: 'old' | 'new'): string {
  const label = side === 'old' ? diff.old_label : diff.new_label;
  const id = side === 'old' ? diff.old_id : diff.new_id;
  return label ?? (id != null ? `ID ${id}` : '—');
}

function isUnchanged(diff: SingleValueDiff): boolean {
  if (diff.unchanged != null) return true;
  return diff.old_id === diff.new_id;
}

export function SingleValueRow({ label, diff }: { label: string; diff: SingleValueDiff }) {
  if (isUnchanged(diff)) {
    return (
      <div className="grid gap-1 border-b border-stitch-border py-3 last:border-0 sm:grid-cols-[10rem_1fr]">
        <span className="text-xs font-bold text-stitch-muted">{label}</span>
        <span className="text-xs text-stitch-muted">
          {diff.unchanged_label ?? (diff.unchanged != null ? `ID ${diff.unchanged}` : '—')} (unchanged)
        </span>
      </div>
    );
  }
  return (
    <div className="grid gap-1 border-b border-stitch-border py-3 last:border-0 sm:grid-cols-[10rem_1fr]">
      <span className="text-xs font-bold text-stitch-muted">{label}</span>
      <span className="text-xs">
        <span className="text-red-700 line-through dark:text-red-300">{valueLabel(diff, 'old')}</span>
        <span className="mx-2 text-stitch-muted">→</span>
        <span className="text-emerald-700 dark:text-emerald-300">{valueLabel(diff, 'new')}</span>
      </span>
    </div>
  );
}

export function CustomFieldRow({ diff }: { diff: CustomFieldDiff }) {
  const oldValue = diff.old_value?.trim() || '—';
  const newValue = diff.new_value?.trim() || '—';
  return (
    <div className="grid gap-1 border-b border-stitch-border py-3 last:border-0 sm:grid-cols-[10rem_1fr]">
      <span className="text-xs font-bold text-stitch-muted">{diff.label}</span>
      {diff.unchanged ? (
        <span className="text-xs text-stitch-muted">{oldValue} (unchanged)</span>
      ) : (
        <span className="text-xs">
          <span className="text-red-700 line-through dark:text-red-300">{oldValue}</span>
          <span className="mx-2 text-stitch-muted">→</span>
          <span className="text-emerald-700 dark:text-emerald-300">{newValue}</span>
        </span>
      )}
    </div>
  );
}
