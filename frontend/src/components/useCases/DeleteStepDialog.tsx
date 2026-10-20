import { useEffect, useState } from 'react';
import Dialog from '@/components/Dialog';

export type DependentBranch = { id: string; label: string | null; title: string | null };

type Props = {
  open: boolean;
  stepLabel: string;
  stepText: string;
  branches: DependentBranch[];
  /** Main steps the branches can move to (the step itself excluded). */
  targets: { id: string; label: string; text: string }[];
  busy: boolean;
  onCancel: () => void;
  onConfirm: (dependents: 'cascade' | { rebranch_to: string }) => void;
};

/** Deleting a step that branches start from or rejoin at (screen 6 of #388). */
export default function DeleteStepDialog({
  open,
  stepLabel,
  stepText,
  branches,
  targets,
  busy,
  onCancel,
  onConfirm,
}: Props) {
  const [choice, setChoice] = useState<'rebranch' | 'cascade'>('rebranch');
  const [target, setTarget] = useState('');

  const firstTarget = targets[0]?.id ?? '';
  useEffect(() => {
    if (!open) return;
    setChoice(firstTarget ? 'rebranch' : 'cascade');
    setTarget(firstTarget);
  }, [open, firstTarget]);

  const names = branches.map((b) => b.label ?? b.id).join(', ');
  const plural = branches.length > 1;

  return (
    <Dialog open={open} onClose={onCancel} title={`Delete step ${stepLabel}?`} subtitle={`“${stepText}”`}>
      <ul className="mb-4 divide-y divide-stitch-border rounded-lg border border-stitch-border text-sm">
        {branches.map((b) => (
          <li key={b.id} className="flex items-center gap-2 px-3 py-2 text-stitch-fg">
            <span className="material-symbols-outlined text-base text-sky-700">alt_route</span>
            Branch <strong>{b.label}</strong>
            {b.title ? <span className="text-stitch-muted">· {b.title}</span> : null} depends on it
          </li>
        ))}
      </ul>
      <fieldset className="space-y-2 text-sm">
        <legend className="mb-2 text-[10px] font-bold uppercase tracking-[0.12em] text-stitch-muted">
          What should happen to {names}?
        </legend>
        <label
          className={`flex items-start gap-2 rounded-lg border px-3 py-2 ${
            choice === 'rebranch' ? 'border-2 border-stitch-accent bg-stitch-accent/5' : 'border-stitch-border'
          } ${targets.length === 0 ? 'opacity-50' : ''}`}
        >
          <input
            type="radio"
            name="dependents"
            checked={choice === 'rebranch'}
            disabled={targets.length === 0}
            onChange={() => setChoice('rebranch')}
            className="mt-1"
          />
          <span className="flex-1">
            <strong>Attach {plural ? 'them' : 'it'} to another step</strong>
            <select
              value={target}
              aria-label="Step to attach the branches to"
              onChange={(e) => {
                setTarget(e.target.value);
                setChoice('rebranch');
              }}
              className="mt-1 block w-full rounded-md border border-stitch-border bg-stitch-elevated px-2 py-1 text-xs text-stitch-fg"
            >
              {targets.map((t) => (
                <option key={t.id} value={t.id}>
                  Step {t.label}: {t.text.length > 60 ? `${t.text.slice(0, 60)}…` : t.text}
                </option>
              ))}
            </select>
          </span>
        </label>
        <label
          className={`flex items-start gap-2 rounded-lg border px-3 py-2 ${
            choice === 'cascade' ? 'border-2 border-stitch-accent bg-stitch-accent/5' : 'border-stitch-border'
          }`}
        >
          <input
            type="radio"
            name="dependents"
            checked={choice === 'cascade'}
            onChange={() => setChoice('cascade')}
            className="mt-1"
          />
          <span>
            <strong>Delete {plural ? 'them' : 'it'} too</strong>
            <br />
            <span className="text-xs text-stitch-muted">Their steps are removed with them.</span>
          </span>
        </label>
      </fieldset>
      <div className="mt-5 flex justify-end gap-2 border-t border-stitch-border pt-4">
        <button
          type="button"
          onClick={onCancel}
          className="rounded-md border border-stitch-border px-3 py-1.5 text-xs font-semibold text-stitch-fg"
        >
          Cancel
        </button>
        <button
          type="button"
          disabled={busy || (choice === 'rebranch' && !target)}
          onClick={() => onConfirm(choice === 'cascade' ? 'cascade' : { rebranch_to: target })}
          className="inline-flex items-center gap-1.5 rounded-md bg-stitch-danger px-3 py-1.5 text-xs font-semibold text-white disabled:opacity-50"
        >
          <span className="material-symbols-outlined text-sm">delete</span>
          Delete step
        </button>
      </div>
    </Dialog>
  );
}
