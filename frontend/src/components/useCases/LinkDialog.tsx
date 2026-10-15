import { FormEvent, useEffect, useMemo, useState } from 'react';
import { listRequirements, listVerificationsByProject } from '@/api/client';
import type { Requirement, Verification } from '@/api/types';
import Dialog from '@/components/Dialog';

export type LinkableElement = { id: string; label: string; text: string };

type Props = {
  open: boolean;
  projectId: number;
  elements: LinkableElement[];
  /** Pre-selected element; `null` links the whole use case. */
  elementId: string | null;
  busy: boolean;
  error: string | null;
  onClose: () => void;
  onCreate: (body: {
    element_id: string | null;
    requirement_id?: number;
    verification_id?: number;
    rationale: string | null;
  }) => void;
};

/** Link the use case, or one of its steps, to a requirement or verification. */
export default function LinkDialog({ open, projectId, elements, elementId, busy, error, onClose, onCreate }: Props) {
  const [kind, setKind] = useState<'requirement' | 'verification'>('requirement');
  const [element, setElement] = useState<string>('');
  const [query, setQuery] = useState('');
  const [target, setTarget] = useState<number | null>(null);
  const [rationale, setRationale] = useState('');
  const [requirements, setRequirements] = useState<Requirement[]>([]);
  const [verifications, setVerifications] = useState<Verification[]>([]);
  const [loadError, setLoadError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    setElement(elementId ?? '');
    setQuery('');
    setTarget(null);
    setRationale('');
    setLoadError(null);
    Promise.all([listRequirements(projectId), listVerificationsByProject(projectId)])
      .then(([r, v]) => {
        setRequirements(r);
        setVerifications(v);
      })
      .catch((e: unknown) => setLoadError(e instanceof Error ? e.message : 'Failed to load'));
  }, [open, elementId, projectId]);

  const options = useMemo(() => {
    const q = query.trim().toLowerCase();
    const all =
      kind === 'requirement'
        ? requirements.map((r) => ({ id: r.id, code: r.reference_code, title: r.title }))
        : verifications.map((v) => ({ id: v.id, code: v.reference_code, title: v.name }));
    return all
      .filter((o) => !q || o.code.toLowerCase().includes(q) || o.title.toLowerCase().includes(q))
      .slice(0, 50);
  }, [kind, query, requirements, verifications]);

  function submit(e: FormEvent) {
    e.preventDefault();
    if (target == null) return;
    onCreate({
      element_id: element || null,
      ...(kind === 'requirement' ? { requirement_id: target } : { verification_id: target }),
      rationale: rationale.trim() || null,
    });
  }

  const field = 'w-full rounded-md border border-stitch-border bg-stitch-elevated px-3 py-2 text-sm text-stitch-fg';

  return (
    <Dialog open={open} onClose={onClose} title="Link a requirement or verification">
      <form onSubmit={submit} className="space-y-3">
        <label className="block">
          <span className="mb-1 block text-[10px] font-bold uppercase text-stitch-muted">Link from</span>
          <select value={element} onChange={(e) => setElement(e.target.value)} className={field}>
            <option value="">The whole use case</option>
            {elements.map((el) => (
              <option key={el.id} value={el.id}>
                {el.label}: {el.text.length > 60 ? `${el.text.slice(0, 60)}…` : el.text}
              </option>
            ))}
          </select>
        </label>
        <div role="radiogroup" aria-label="Link to" className="flex gap-2">
          {(['requirement', 'verification'] as const).map((k) => (
            <button
              key={k}
              type="button"
              role="radio"
              aria-checked={kind === k}
              onClick={() => {
                setKind(k);
                setTarget(null);
              }}
              className={`rounded-md border px-3 py-1.5 text-xs font-semibold ${
                kind === k
                  ? 'border-stitch-accent bg-stitch-accent/10 text-stitch-accent'
                  : 'border-stitch-border text-stitch-fg'
              }`}
            >
              {k === 'requirement' ? 'Requirement' : 'Verification'}
            </button>
          ))}
        </div>
        <input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Filter by code or title…"
          aria-label="Filter targets"
          className={field}
        />
        <ul
          role="listbox"
          aria-label={kind === 'requirement' ? 'Requirements' : 'Verifications'}
          className="max-h-48 overflow-y-auto rounded-md border border-stitch-border"
        >
          {options.length === 0 ? (
            <li className="px-3 py-2 text-xs text-stitch-muted">{loadError ?? 'Nothing matches.'}</li>
          ) : (
            options.map((o) => (
              <li
                key={o.id}
                role="option"
                aria-selected={target === o.id}
                onClick={() => setTarget(o.id)}
                className={`flex cursor-pointer gap-2 px-3 py-1.5 text-xs ${
                  target === o.id ? 'bg-stitch-accent/10' : 'hover:bg-stitch-elevated'
                }`}
              >
                <span className="font-mono font-semibold text-stitch-accent">{o.code}</span>
                <span className="truncate text-stitch-fg">{o.title}</span>
              </li>
            ))
          )}
        </ul>
        <label className="block">
          <span className="mb-1 block text-[10px] font-bold uppercase text-stitch-muted">Rationale (optional)</span>
          <input value={rationale} onChange={(e) => setRationale(e.target.value)} className={field} />
        </label>
        {error ? (
          <p role="alert" className="text-sm text-red-700 dark:text-red-300">
            {error}
          </p>
        ) : null}
        <div className="flex justify-end gap-2">
          <button type="button" onClick={onClose} className="rounded-md border border-stitch-border px-3 py-1.5 text-xs font-semibold text-stitch-fg">
            Cancel
          </button>
          <button
            type="submit"
            disabled={busy || target == null}
            className="rounded-md bg-stitch-accent px-3 py-1.5 text-xs font-bold text-stitch-on-accent disabled:opacity-50"
          >
            Link
          </button>
        </div>
      </form>
    </Dialog>
  );
}
