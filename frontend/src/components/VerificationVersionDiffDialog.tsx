import { useEffect, useMemo, useRef, useState } from 'react';
import { compareVerificationSnapshotsByProject } from '@/api/client';
import type { VerificationSnapshot, VerificationVersionDiff } from '@/api/types';
import { SingleValueRow, TextDiffField } from '@/components/versionDiffUi';

type VersionPair = {
  oldVersionId?: number;
  newVersionId?: number;
};

type VerificationVersionDiffDialogProps = {
  open: boolean;
  onClose: () => void;
  projectId: number;
  verificationId: number;
  snapshots: VerificationSnapshot[];
  initialPair?: VersionPair | null;
};

function snapshotTime(snapshot: VerificationSnapshot): number {
  const parsed = new Date(snapshot.created_at).getTime();
  return Number.isFinite(parsed) ? parsed : 0;
}

export function VerificationDiffContent({ diff }: { diff: VerificationVersionDiff }) {
  return (
    <div className="space-y-5">
      <TextDiffField label="Name" diff={diff.text.name} />
      <TextDiffField label="Description" diff={diff.text.description} />
      <TextDiffField label="Source" diff={diff.text.source} />
      <TextDiffField label="Reference" diff={diff.text.reference_code} />
      <section className="rounded-xl border border-stitch-border bg-stitch-canvas/60 px-4">
        <h3 className="pt-4 text-xs font-bold uppercase tracking-widest text-stitch-fg">Metadata</h3>
        <SingleValueRow label="Status" diff={diff.metadata.status} />
        <SingleValueRow label="Verification type" diff={diff.metadata.verification_method} />
        <SingleValueRow label="Parent" diff={diff.metadata.parent} />
      </section>
    </div>
  );
}

export default function VerificationVersionDiffDialog({
  open,
  onClose,
  projectId,
  verificationId,
  snapshots,
  initialPair,
}: VerificationVersionDiffDialogProps) {
  const closeButtonRef = useRef<HTMLButtonElement>(null);
  const orderedSnapshots = useMemo(
    () =>
      [...snapshots].sort(
        (a, b) => snapshotTime(a) - snapshotTime(b) || a.id - b.id,
      ),
    [snapshots],
  );
  const indexById = useMemo(
    () => new Map(orderedSnapshots.map((snapshot, index) => [snapshot.id, index])),
    [orderedSnapshots],
  );
  const [oldVersionId, setOldVersionId] = useState<number | null>(null);
  const [newVersionId, setNewVersionId] = useState<number | null>(null);
  const [diff, setDiff] = useState<VerificationVersionDiff | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open || orderedSnapshots.length < 2) return;
    const defaultOld = orderedSnapshots.at(-2)?.id ?? null;
    const defaultNew = orderedSnapshots.at(-1)?.id ?? null;
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
  }, [indexById, initialPair, open, orderedSnapshots]);

  useEffect(() => {
    if (!open || oldVersionId == null || newVersionId == null || oldVersionId === newVersionId) {
      return;
    }
    let alive = true;
    setLoading(true);
    setError(null);
    setDiff(null);
    compareVerificationSnapshotsByProject(
      projectId,
      verificationId,
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
  }, [newVersionId, oldVersionId, open, projectId, verificationId]);

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

  const snapshotLabel = (snapshot: VerificationSnapshot) => {
    const index = indexById.get(snapshot.id) ?? 0;
    const date = new Date(snapshot.created_at);
    const dateLabel = Number.isNaN(date.getTime()) ? snapshot.created_at : date.toLocaleString();
    const prefix = snapshot.id === 0 ? 'Before recorded history' : `v${index + 1}`;
    return `${prefix} — ${dateLabel}`;
  };

  const updateOld = (id: number) => {
    const selectedIndex = indexById.get(id) ?? 0;
    const newIndex = newVersionId == null ? -1 : (indexById.get(newVersionId) ?? -1);
    setOldVersionId(id);
    if (selectedIndex >= newIndex) {
      setNewVersionId(orderedSnapshots[Math.min(selectedIndex + 1, orderedSnapshots.length - 1)].id);
    }
  };
  const updateNew = (id: number) => {
    const selectedIndex = indexById.get(id) ?? 0;
    const oldIndex = oldVersionId == null ? orderedSnapshots.length : (indexById.get(oldVersionId) ?? 0);
    setNewVersionId(id);
    if (selectedIndex <= oldIndex) {
      setOldVersionId(orderedSnapshots[Math.max(0, selectedIndex - 1)].id);
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
        aria-labelledby="verification-version-diff-title"
        className="flex max-h-full w-full max-w-5xl flex-col overflow-hidden rounded-2xl border border-stitch-border bg-stitch-surface shadow-2xl"
      >
        <header className="flex shrink-0 items-start justify-between gap-4 border-b border-stitch-border px-5 py-4 md:px-6">
          <div>
            <h2 id="verification-version-diff-title" className="font-headline text-lg font-bold text-stitch-fg">
              Compare verification versions
            </h2>
            <p className="mt-1 text-xs text-stitch-muted">
              Snapshots come from the audit log. Removed values are red; additions are green.
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

        {orderedSnapshots.length < 2 ? (
          <div className="p-8 text-center text-sm text-stitch-muted">
            At least two saved versions are required for comparison. Edit and save the verification
            to create a second snapshot.
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
                  {orderedSnapshots.slice(0, -1).map((snapshot) => (
                    <option key={snapshot.id} value={snapshot.id}>
                      {snapshotLabel(snapshot)}
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
                  {orderedSnapshots.slice(1).map((snapshot) => (
                    <option key={snapshot.id} value={snapshot.id}>
                      {snapshotLabel(snapshot)}
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
                <VerificationDiffContent diff={diff} />
              ) : null}
            </div>
          </>
        )}
      </div>
    </div>
  );
}
