import { type FormEvent, useEffect, useState } from 'react';
import { getRequirementCloseOut, putRequirementCompliance } from '@/api/client';
import type { Compliance, RequirementCloseOut } from '@/api/types';

type Props = {
  projectId: number;
  requirementId: number;
  /** `is_project_reviewer`: only reviewers assess compliance. */
  canAssess: boolean;
  csrfToken: string;
  className?: string;
};

export const COMPLIANCE_LABELS: Record<Compliance, string> = {
  C: 'Compliant',
  PC: 'Partially compliant',
  NC: 'Non-compliant',
};

const inputCls =
  'w-full rounded-md border border-stitch-border bg-stitch-surface px-2 py-1.5 text-xs text-stitch-fg disabled:opacity-70';

/**
 * Requirement page: verification close-out (issue #353). Shows whether the
 * requirement is closed and why, and lets project reviewers record the
 * compliance assessment (C / PC / NC) that closes it.
 */
export default function RequirementCloseOutPanel({
  projectId,
  requirementId,
  canAssess,
  csrfToken,
  className = '',
}: Props) {
  const [data, setData] = useState<RequirementCloseOut | null>(null);
  const [compliance, setCompliance] = useState<Compliance | ''>('');
  const [note, setNote] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    let cancelled = false;
    getRequirementCloseOut(projectId, requirementId)
      .then((d) => {
        if (cancelled) return;
        setData(d);
        setCompliance(d.compliance ?? '');
        setNote(d.note ?? '');
      })
      .catch((e: unknown) => {
        if (!cancelled) setError(e instanceof Error ? e.message : 'Could not load the close-out.');
      });
    return () => {
      cancelled = true;
    };
  }, [projectId, requirementId]);

  const dirty =
    data != null && (compliance !== (data.compliance ?? '') || note.trim() !== (data.note ?? ''));

  async function onSave(e: FormEvent) {
    e.preventDefault();
    if (!dirty || busy) return;
    setBusy(true);
    setError(null);
    setSaved(false);
    try {
      const d = await putRequirementCompliance(
        projectId,
        requirementId,
        compliance === '' ? null : compliance,
        note.trim() === '' ? null : note.trim(),
        csrfToken,
      );
      setData(d);
      setCompliance(d.compliance ?? '');
      setNote(d.note ?? '');
      setSaved(true);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Could not save the assessment.');
    } finally {
      setBusy(false);
    }
  }

  const closed = data?.close_out.status === 'closed';

  return (
    <section
      aria-label="Verification close-out"
      className={`bg-stitch-surface rounded-xl border border-stitch-border p-6 shadow-stitch ${className}`}
    >
      <div className="flex items-center gap-2 mb-3">
        <span className="material-symbols-outlined text-stitch-accent text-xl">verified</span>
        <h2 className="text-sm font-bold font-headline text-stitch-accent">Verification close-out</h2>
      </div>
      {data ? (
        <>
          <p className="flex flex-wrap items-center gap-2 text-xs">
            <span
              className={`rounded px-2 py-0.5 text-[10px] font-bold uppercase tracking-wide ${
                closed
                  ? 'bg-emerald-500/15 text-emerald-800 dark:text-emerald-200'
                  : 'bg-indigo-500/15 text-indigo-800 dark:text-indigo-200'
              }`}
            >
              {closed ? 'Closed' : 'Open'}
            </span>
            <span className="text-stitch-muted">{data.close_out.reason}</span>
          </p>
          <form onSubmit={onSave} className="mt-4 space-y-2">
            <label className="block text-[10px] uppercase tracking-widest text-stitch-muted font-bold">
              Compliance
              <select
                className={`${inputCls} mt-1`}
                value={compliance}
                disabled={!canAssess || busy}
                onChange={(e) => {
                  setSaved(false);
                  setCompliance(e.target.value as Compliance | '');
                }}
              >
                <option value="">Not assessed</option>
                {(Object.keys(COMPLIANCE_LABELS) as Compliance[]).map((c) => (
                  <option key={c} value={c}>
                    {c} · {COMPLIANCE_LABELS[c]}
                  </option>
                ))}
              </select>
            </label>
            <label className="block text-[10px] uppercase tracking-widest text-stitch-muted font-bold">
              Note
              <textarea
                className={`${inputCls} mt-1 normal-case tracking-normal font-normal`}
                rows={2}
                value={note}
                disabled={!canAssess || busy}
                placeholder={canAssess ? 'Evidence, waiver or NCR reference' : ''}
                onChange={(e) => {
                  setSaved(false);
                  setNote(e.target.value);
                }}
              />
            </label>
            {canAssess ? (
              <div className="flex items-center gap-3">
                <button
                  type="submit"
                  disabled={!dirty || busy}
                  className="rounded-md bg-stitch-accent px-3 py-1.5 text-xs font-semibold text-white disabled:opacity-50"
                >
                  {busy ? 'Saving…' : 'Save assessment'}
                </button>
                {saved ? <span className="text-xs text-emerald-700 dark:text-emerald-300">Saved</span> : null}
              </div>
            ) : (
              <p className="text-[11px] text-stitch-muted">Only project reviewers can assess compliance.</p>
            )}
          </form>
        </>
      ) : error ? null : (
        <p className="text-xs text-stitch-muted">Loading…</p>
      )}
      {error ? (
        <p role="alert" className="mt-2 text-xs text-red-700 dark:text-red-300">
          {error}
        </p>
      ) : null}
    </section>
  );
}
