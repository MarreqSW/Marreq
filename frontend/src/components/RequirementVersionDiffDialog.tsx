import { useEffect, useMemo, useRef, useState } from 'react';
import { compareRequirementVersionsByProject } from '@/api/client';
import type {
  CustomFieldDiff,
  RequirementDiff,
  RequirementVersion,
  SingleValueDiff,
  TextDiffResult,
  VerificationMethodDiff,
} from '@/api/types';

type VersionPair = {
  oldVersionId?: number;
  newVersionId?: number;
};

type RequirementVersionDiffDialogProps = {
  open: boolean;
  onClose: () => void;
  projectId: number;
  requirementId: number;
  versions: RequirementVersion[];
  initialPair?: VersionPair | null;
};

function versionTime(version: RequirementVersion): number {
  const parsed = new Date(version.created_at).getTime();
  return Number.isFinite(parsed) ? parsed : 0;
}

function meaningful(lines: string[]): string[] {
  return lines.filter((line) => line.length > 0);
}

function TextDiffField({ label, diff }: { label: string; diff: TextDiffResult }) {
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

function SingleValueRow({ label, diff }: { label: string; diff: SingleValueDiff }) {
  if (diff.unchanged != null) {
    return (
      <div className="grid gap-1 border-b border-stitch-border py-3 last:border-0 sm:grid-cols-[10rem_1fr]">
        <span className="text-xs font-bold text-stitch-muted">{label}</span>
        <span className="text-xs text-stitch-muted">
          {diff.unchanged_label ?? `ID ${diff.unchanged}`} (unchanged)
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

function methodLabels(diff: VerificationMethodDiff, key: 'added' | 'removed' | 'unchanged') {
  const labels = diff[`${key}_labels`];
  const ids = diff[`${key}_ids`];
  return labels?.length ? labels : ids.map((id) => `ID ${id}`);
}

function VerificationMethodsRow({ diff }: { diff: VerificationMethodDiff }) {
  const removed = methodLabels(diff, 'removed');
  const added = methodLabels(diff, 'added');
  const unchanged = methodLabels(diff, 'unchanged');
  return (
    <div className="grid gap-2 border-b border-stitch-border py-3 sm:grid-cols-[10rem_1fr]">
      <span className="text-xs font-bold text-stitch-muted">Verification methods</span>
      <div className="space-y-1 text-xs">
        {removed.length ? (
          <p className="text-red-700 dark:text-red-300">Removed: {removed.join(', ')}</p>
        ) : null}
        {added.length ? (
          <p className="text-emerald-700 dark:text-emerald-300">Added: {added.join(', ')}</p>
        ) : null}
        {unchanged.length ? <p className="text-stitch-muted">Unchanged: {unchanged.join(', ')}</p> : null}
        {!removed.length && !added.length && !unchanged.length ? (
          <p className="text-stitch-muted">No methods in either version.</p>
        ) : null}
      </div>
    </div>
  );
}

function CustomFieldRow({ diff }: { diff: CustomFieldDiff }) {
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

function DiffContent({ diff }: { diff: RequirementDiff }) {
  return (
    <div className="space-y-5">
      <TextDiffField label="Title" diff={diff.text.title} />
      <TextDiffField label="Statement" diff={diff.text.description} />
      <TextDiffField label="Justification" diff={diff.text.justification} />
      <section className="rounded-xl border border-stitch-border bg-stitch-canvas/60 px-4">
        <h3 className="pt-4 text-xs font-bold uppercase tracking-widest text-stitch-fg">Metadata</h3>
        <SingleValueRow label="Status" diff={diff.metadata.status} />
        <SingleValueRow label="Category" diff={diff.metadata.category} />
        <SingleValueRow label="Applicability" diff={diff.metadata.applicability} />
        <VerificationMethodsRow diff={diff.metadata.verification} />
        {diff.metadata.custom_fields.length > 0 ? (
          <div className="py-3">
            <h4 className="mb-1 text-[10px] font-bold uppercase tracking-widest text-stitch-muted">
              Custom fields
            </h4>
            {diff.metadata.custom_fields.map((field) => (
              <CustomFieldRow key={field.field_id} diff={field} />
            ))}
          </div>
        ) : null}
      </section>
    </div>
  );
}

export default function RequirementVersionDiffDialog({
  open,
  onClose,
  projectId,
  requirementId,
  versions,
  initialPair,
}: RequirementVersionDiffDialogProps) {
  const closeButtonRef = useRef<HTMLButtonElement>(null);
  const orderedVersions = useMemo(
    () =>
      [...versions].sort(
        (a, b) => versionTime(a) - versionTime(b) || a.id - b.id,
      ),
    [versions],
  );
  const indexById = useMemo(
    () => new Map(orderedVersions.map((version, index) => [version.id, index])),
    [orderedVersions],
  );
  const [oldVersionId, setOldVersionId] = useState<number | null>(null);
  const [newVersionId, setNewVersionId] = useState<number | null>(null);
  const [diff, setDiff] = useState<RequirementDiff | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open || orderedVersions.length < 2) return;
    const defaultOld = orderedVersions.at(-2)?.id ?? null;
    const defaultNew = orderedVersions.at(-1)?.id ?? null;
    const requestedOld = initialPair?.oldVersionId;
    const requestedNew = initialPair?.newVersionId;
    const oldIndex = requestedOld == null ? undefined : indexById.get(requestedOld);
    const newIndex = requestedNew == null ? undefined : indexById.get(requestedNew);
    if (oldIndex != null && newIndex != null && oldIndex !== newIndex) {
      if (oldIndex < newIndex) {
        setOldVersionId(requestedOld ?? null);
        setNewVersionId(requestedNew ?? null);
      } else {
        setOldVersionId(requestedNew ?? null);
        setNewVersionId(requestedOld ?? null);
      }
    } else {
      setOldVersionId(defaultOld);
      setNewVersionId(defaultNew);
    }
  }, [indexById, initialPair, open, orderedVersions]);

  useEffect(() => {
    if (!open || oldVersionId == null || newVersionId == null || oldVersionId === newVersionId) {
      return;
    }
    let alive = true;
    setLoading(true);
    setError(null);
    setDiff(null);
    compareRequirementVersionsByProject(
      projectId,
      requirementId,
      oldVersionId,
      newVersionId,
    )
      .then((result) => {
        if (alive) setDiff(result);
      })
      .catch((reason) => {
        if (alive) {
          setError(reason instanceof Error ? reason.message : 'Failed to compare versions');
        }
      })
      .finally(() => {
        if (alive) setLoading(false);
      });
    return () => {
      alive = false;
    };
  }, [newVersionId, oldVersionId, open, projectId, requirementId]);

  useEffect(() => {
    if (!open) return;
    const previousOverflow = document.body.style.overflow;
    document.body.style.overflow = 'hidden';
    closeButtonRef.current?.focus();
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', onKeyDown);
    return () => {
      document.body.style.overflow = previousOverflow;
      window.removeEventListener('keydown', onKeyDown);
    };
  }, [onClose, open]);

  if (!open) return null;

  const versionLabel = (version: RequirementVersion) => {
    const index = indexById.get(version.id) ?? 0;
    const date = new Date(version.created_at);
    const dateLabel = Number.isNaN(date.getTime()) ? version.created_at : date.toLocaleString();
    return `v${index + 1} — ${dateLabel}`;
  };

  const updateOld = (id: number) => {
    const selectedIndex = indexById.get(id) ?? 0;
    const newIndex = newVersionId == null ? -1 : (indexById.get(newVersionId) ?? -1);
    setOldVersionId(id);
    if (selectedIndex >= newIndex) {
      setNewVersionId(orderedVersions[Math.min(selectedIndex + 1, orderedVersions.length - 1)].id);
    }
  };
  const updateNew = (id: number) => {
    const selectedIndex = indexById.get(id) ?? 0;
    const oldIndex = oldVersionId == null ? orderedVersions.length : (indexById.get(oldVersionId) ?? 0);
    setNewVersionId(id);
    if (selectedIndex <= oldIndex) {
      setOldVersionId(orderedVersions[Math.max(0, selectedIndex - 1)].id);
    }
  };

  return (
    <div
      className="fixed inset-0 z-[80] flex items-center justify-center bg-black/55 p-3 backdrop-blur-sm md:p-8"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="requirement-version-diff-title"
        className="flex max-h-full w-full max-w-5xl flex-col overflow-hidden rounded-2xl border border-stitch-border bg-stitch-surface shadow-2xl"
      >
        <header className="flex shrink-0 items-start justify-between gap-4 border-b border-stitch-border px-5 py-4 md:px-6">
          <div>
            <h2 id="requirement-version-diff-title" className="font-headline text-lg font-bold text-stitch-fg">
              Compare requirement versions
            </h2>
            <p className="mt-1 text-xs text-stitch-muted">
              Removed values are red; additions are green. Versions are compared oldest to newest.
            </p>
          </div>
          <button
            ref={closeButtonRef}
            type="button"
            aria-label="Close version comparison"
            onClick={onClose}
            className="rounded-lg p-1.5 text-stitch-muted transition-colors hover:bg-stitch-elevated hover:text-stitch-fg"
          >
            <span className="material-symbols-outlined">close</span>
          </button>
        </header>

        {orderedVersions.length < 2 ? (
          <div className="p-8 text-center text-sm text-stitch-muted">
            At least two saved versions are required for comparison.
          </div>
        ) : (
          <>
            <div className="grid shrink-0 gap-3 border-b border-stitch-border bg-stitch-elevated/60 px-5 py-4 sm:grid-cols-2 md:px-6">
              <label className="text-[10px] font-bold uppercase tracking-widest text-stitch-muted">
                Older version
                <select
                  value={oldVersionId ?? ''}
                  onChange={(event) => updateOld(Number(event.target.value))}
                  className="mt-1 block w-full rounded-lg border border-stitch-border bg-stitch-surface px-3 py-2 text-sm font-normal normal-case tracking-normal text-stitch-fg"
                >
                  {orderedVersions.slice(0, -1).map((version) => (
                    <option key={version.id} value={version.id}>
                      {versionLabel(version)}
                    </option>
                  ))}
                </select>
              </label>
              <label className="text-[10px] font-bold uppercase tracking-widest text-stitch-muted">
                Newer version
                <select
                  value={newVersionId ?? ''}
                  onChange={(event) => updateNew(Number(event.target.value))}
                  className="mt-1 block w-full rounded-lg border border-stitch-border bg-stitch-surface px-3 py-2 text-sm font-normal normal-case tracking-normal text-stitch-fg"
                >
                  {orderedVersions.slice(1).map((version) => (
                    <option key={version.id} value={version.id}>
                      {versionLabel(version)}
                    </option>
                  ))}
                </select>
              </label>
            </div>
            <div className="min-h-0 flex-1 overflow-y-auto p-5 md:p-6">
              {loading ? (
                <div className="py-12 text-center text-sm text-stitch-muted">Comparing versions…</div>
              ) : error ? (
                <div className="rounded-xl border border-red-500/30 bg-red-500/10 p-4 text-sm text-red-900 dark:text-red-100">
                  {error}
                </div>
              ) : diff ? (
                <DiffContent diff={diff} />
              ) : null}
            </div>
          </>
        )}
      </div>
    </div>
  );
}
