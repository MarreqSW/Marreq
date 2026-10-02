import { type FormEvent, useEffect, useId, useState } from 'react';
import { getVerificationControl, putVerificationControl } from '@/api/client';
import type { VerificationControl } from '@/api/types';

type Props = {
  projectId: number;
  verificationId: number;
  /** `edit_requirements`. */
  canEdit: boolean;
  csrfToken: string;
  className?: string;
};

type Draft = { level: string; stage: string; evidence: string };

function toDraft(c: VerificationControl): Draft {
  return {
    level: c.verification_level ?? '',
    stage: c.verification_stage ?? '',
    evidence: c.evidence_reference ?? '',
  };
}

const labelCls = 'block text-[10px] uppercase tracking-widest text-stitch-muted font-bold mb-1';
const inputCls =
  'w-full rounded-md border border-stitch-border bg-stitch-surface px-3 py-2 text-sm text-stitch-fg disabled:opacity-70 disabled:bg-stitch-elevated/50';

/**
 * Verification page: verification level, stage and close-out evidence
 * (issue #353), reported in the Verification Control Document.
 */
export default function VerificationControlPanel({
  projectId,
  verificationId,
  canEdit,
  csrfToken,
  className = '',
}: Props) {
  const ids = useId();
  const [control, setControl] = useState<VerificationControl | null>(null);
  const [draft, setDraft] = useState<Draft>({ level: '', stage: '', evidence: '' });
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    let cancelled = false;
    getVerificationControl(projectId, verificationId)
      .then((c) => {
        if (cancelled) return;
        setControl(c);
        setDraft(toDraft(c));
      })
      .catch((e: unknown) => {
        if (!cancelled) setError(e instanceof Error ? e.message : 'Could not load the verification control data.');
      });
    return () => {
      cancelled = true;
    };
  }, [projectId, verificationId]);

  const original = control ? toDraft(control) : null;
  const dirty =
    original != null &&
    (draft.level.trim() !== original.level ||
      draft.stage.trim() !== original.stage ||
      draft.evidence.trim() !== original.evidence);

  const set = (patch: Partial<Draft>) => {
    setSaved(false);
    setDraft((d) => ({ ...d, ...patch }));
  };

  async function onSave(e: FormEvent) {
    e.preventDefault();
    if (!dirty || busy) return;
    setBusy(true);
    setError(null);
    try {
      const c = await putVerificationControl(
        projectId,
        verificationId,
        {
          verification_level: draft.level.trim(),
          verification_stage: draft.stage.trim(),
          evidence_reference: draft.evidence.trim(),
        },
        csrfToken,
      );
      setControl(c);
      setDraft(toDraft(c));
      setSaved(true);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Could not save.');
    } finally {
      setBusy(false);
    }
  }

  return (
    <section
      aria-label="Verification control"
      className={`bg-stitch-surface rounded-xl border border-stitch-border shadow-stitch p-6 md:p-8 ${className}`}
    >
      <h2 className="text-sm font-bold font-headline text-stitch-fg">Verification control</h2>
      <p className="text-[10px] text-stitch-muted mt-1 mb-4">
        Level, stage and evidence reported in the Verification Control Document.
      </p>
      {control ? (
        <form onSubmit={onSave} className="grid gap-4 md:grid-cols-3">
          <label>
            <span className={labelCls}>Level</span>
            <input
              className={inputCls}
              list={`${ids}-levels`}
              maxLength={40}
              value={draft.level}
              disabled={!canEdit || busy}
              placeholder={canEdit ? 'e.g. Subsystem' : ''}
              onChange={(e) => set({ level: e.target.value })}
            />
            <datalist id={`${ids}-levels`}>
              {control.suggestions.levels.map((l) => (
                <option key={l} value={l} />
              ))}
            </datalist>
          </label>
          <label>
            <span className={labelCls}>Stage</span>
            <input
              className={inputCls}
              list={`${ids}-stages`}
              maxLength={40}
              value={draft.stage}
              disabled={!canEdit || busy}
              placeholder={canEdit ? 'e.g. QUAL' : ''}
              onChange={(e) => set({ stage: e.target.value })}
            />
            <datalist id={`${ids}-stages`}>
              {control.suggestions.stages.map((s) => (
                <option key={s} value={s} />
              ))}
            </datalist>
          </label>
          <label>
            <span className={labelCls}>Evidence</span>
            <input
              className={inputCls}
              value={draft.evidence}
              disabled={!canEdit || busy}
              placeholder={canEdit ? 'e.g. TR-PWR-002' : ''}
              onChange={(e) => set({ evidence: e.target.value })}
            />
          </label>
          {canEdit ? (
            <div className="md:col-span-3 flex items-center gap-3">
              <button
                type="submit"
                disabled={!dirty || busy}
                className="rounded-md bg-stitch-accent px-4 py-2 text-xs font-semibold text-white disabled:opacity-50"
              >
                {busy ? 'Saving…' : 'Save'}
              </button>
              {saved ? <span className="text-xs text-emerald-700 dark:text-emerald-300">Saved</span> : null}
            </div>
          ) : null}
        </form>
      ) : error ? null : (
        <p className="text-sm text-stitch-muted">Loading…</p>
      )}
      {error ? (
        <p role="alert" className="mt-3 text-xs text-red-700 dark:text-red-300">
          {error}
        </p>
      ) : null}
    </section>
  );
}
