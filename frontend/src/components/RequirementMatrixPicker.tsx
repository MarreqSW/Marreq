import { useMemo, useState } from 'react';
import { Link } from 'react-router-dom';
import type { Requirement } from '@/api/types';

type Props = {
  projectId: number;
  basePath: string;
  requirements: Requirement[];
  selectedIds: number[];
  onChange: (ids: number[]) => void;
  disabled?: boolean;
};

/**
 * Filterable checkbox list of project requirements for traceability matrix (requirement ↔ verification links).
 */
export function RequirementMatrixPicker({
  projectId,
  basePath,
  requirements,
  selectedIds,
  onChange,
  disabled = false,
}: Props) {
  const [q, setQ] = useState('');
  const selectedSet = useMemo(() => new Set(selectedIds), [selectedIds]);
  const sorted = useMemo(
    () =>
      [...requirements].sort((a, b) =>
        (a.reference_code || '').localeCompare(b.reference_code || '', undefined, {
          numeric: true,
        }),
      ),
    [requirements],
  );
  const filtered = useMemo(() => {
    const t = q.trim().toLowerCase();
    if (!t) return sorted;
    return sorted.filter(
      (r) =>
        (r.reference_code || '').toLowerCase().includes(t) ||
        (r.title || '').toLowerCase().includes(t) ||
        String(r.id).includes(t),
    );
  }, [sorted, q]);

  function toggle(id: number) {
    if (disabled) return;
    const next = new Set(selectedIds);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    onChange([...next].sort((a, b) => a - b));
  }

  return (
    <div className="space-y-3">
      <input
        type="search"
        value={q}
        onChange={(e) => setQ(e.target.value)}
        disabled={disabled}
        placeholder="Filter by reference, title, or id…"
        className="w-full text-sm font-medium bg-stitch-elevated border border-stitch-border rounded-md px-2 py-2 text-stitch-fg placeholder:text-stitch-muted/60 focus:border-stitch-accent focus:ring-1 focus:ring-stitch-accent/40 outline-none"
      />
      <div className="max-h-56 overflow-y-auto rounded-md border border-stitch-border bg-stitch-elevated divide-y divide-stitch-border">
        {filtered.length === 0 ? (
          <p className="p-3 text-xs text-stitch-muted">No matching requirements.</p>
        ) : (
          filtered.map((r) => (
            <div
              key={r.id}
              className="flex items-start gap-3 px-3 py-2 hover:bg-white/[0.04]"
            >
              <input
                type="checkbox"
                id={`req-matrix-${r.id}`}
                className="mt-1 rounded border-stitch-border text-stitch-accent"
                checked={selectedSet.has(r.id)}
                onChange={() => toggle(r.id)}
                disabled={disabled}
              />
              <label
                htmlFor={`req-matrix-${r.id}`}
                className={`min-w-0 flex-1 ${disabled ? 'cursor-not-allowed opacity-60' : 'cursor-pointer'}`}
              >
                <span className="font-mono text-stitch-accent text-xs">
                  {r.reference_code || `#${r.id}`}
                </span>
                <span className="block text-sm text-stitch-fg line-clamp-2">{r.title}</span>
              </label>
              <Link
                to={`${basePath}/requirements/${r.id}/edit`}
                onClick={(e) => e.stopPropagation()}
                className="shrink-0 text-[10px] font-bold uppercase tracking-wider text-stitch-muted hover:text-stitch-accent pt-0.5"
              >
                Open
              </Link>
            </div>
          ))
        )}
      </div>
      <p className="text-[10px] text-stitch-muted">
        {selectedIds.length} requirement{selectedIds.length === 1 ? '' : 's'} linked in matrix
      </p>
    </div>
  );
}

function sortedCopy(ids: number[]): number[] {
  return [...ids].sort((a, b) => a - b);
}

export function matrixSelectionEquals(a: number[], b: number[]): boolean {
  const sa = sortedCopy(a);
  const sb = sortedCopy(b);
  if (sa.length !== sb.length) return false;
  return sa.every((v, i) => v === sb[i]);
}
