import type { KeyboardEvent } from 'react';
import type { Dsm } from '@/api/types';
import { DSM_LINK_TYPES, linkTypeMeta } from '@/utils/dsm';

type Props = {
  dsm: Dsm;
  highlightedLoop: number | null;
  onHighlightLoop: (loopIndex: number | null) => void;
  /** Problem selected by a click (focused in the matrix). */
  selectedLoop: number | null;
  selectedFinding: { row: number; col: number } | null;
  onSelectLoop: (loopIndex: number) => void;
  onSelectFinding: (row: number, col: number) => void;
};

/** Clickable panel item (it contains links, so it cannot be a <button>). */
function selectable(selected: boolean, onSelect: () => void) {
  return {
    role: 'button',
    tabIndex: 0,
    'aria-pressed': selected,
    onClick: onSelect,
    onKeyDown: (e: KeyboardEvent) => {
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        onSelect();
      }
    },
  } as const;
}

const sectionTitle =
  'flex justify-between text-[10px] uppercase tracking-widest text-stitch-muted font-bold mb-2';

export default function DsmSidePanel({
  dsm,
  highlightedLoop,
  onHighlightLoop,
  selectedLoop,
  selectedFinding,
  onSelectLoop,
  onSelectFinding,
}: Props) {
  const byId = new Map(dsm.requirements.map((r) => [r.id, r]));
  const code = (id: number) => byId.get(id)?.reference_code ?? `#${id}`;
  const changed = dsm.cells.filter((c) => c.upstream_changed);

  // Plain text, not links: the whole card is the click target that focuses the matrix.
  // Requirements open from the row headers or by clicking a mark.
  const reqCode = (id: number) => (
    <span key={id} className="font-mono text-[11px] font-semibold text-stitch-accent">
      {code(id)}
    </span>
  );
  const hint = <p className="mt-1 text-[10px] text-stitch-subtle">Click to show in the matrix</p>;

  return (
    <aside className="rounded-xl border border-stitch-border bg-stitch-surface p-4 space-y-5 text-xs" aria-label="DSM details">
      <section>
        <h3 className={sectionTitle}>
          Loops <span className="font-mono text-stitch-fg">{dsm.loops.length}</span>
        </h3>
        {dsm.loops.length === 0 ? (
          <p className="text-stitch-muted">No dependency loops for the selected link types.</p>
        ) : (
          <ul className="space-y-2">
            {dsm.loops.map((l, i) => (
              <li
                key={l.requirement_ids.join('-')}
                data-testid={`dsm-loop-${i}`}
                className={`cursor-pointer rounded-lg border p-2.5 focus:outline-2 focus:outline-amber-500 ${
                  selectedLoop === i
                    ? 'border-amber-500 bg-amber-400/20 ring-1 ring-amber-500'
                    : highlightedLoop === i
                      ? 'border-amber-500 bg-amber-400/10'
                      : 'border-stitch-border hover:bg-stitch-higher'
                }`}
                onMouseEnter={() => onHighlightLoop(i)}
                onMouseLeave={() => onHighlightLoop(null)}
                title="Show this loop in the matrix"
                {...selectable(selectedLoop === i, () => onSelectLoop(i))}
              >
                <span className="inline-block rounded bg-amber-400/20 px-1.5 py-0.5 font-mono text-[10px] font-bold text-amber-700 dark:text-amber-300">
                  LOOP {i + 1} · {l.requirement_ids.length} requirements
                </span>
                <p className="mt-1.5 leading-relaxed">
                  {[...l.path, l.path[0]!].map((id, k) => (
                    <span key={`${id}-${k}`}>
                      {k > 0 ? <span className="text-stitch-muted"> → </span> : null}
                      {reqCode(id)}
                    </span>
                  ))}
                </p>
                {hint}
              </li>
            ))}
          </ul>
        )}
      </section>

      <section>
        <h3 className={sectionTitle}>
          Upstream changed <span className="font-mono text-stitch-fg">{changed.length}</span>
        </h3>
        {changed.length === 0 ? (
          <p className="text-stitch-muted">No approved requirement depends on something edited after its approval.</p>
        ) : (
          <ul className="space-y-2">
            {changed.map((c) => {
              const source = dsm.requirements[c.row]!;
              const target = dsm.requirements[c.col]!;
              return (
                <li
                  key={`${c.row}:${c.col}`}
                  data-testid={`dsm-finding-${c.row}-${c.col}`}
                  className={`cursor-pointer rounded-lg border p-2.5 focus:outline-2 focus:outline-stitch-danger ${
                    selectedFinding?.row === c.row && selectedFinding.col === c.col
                      ? 'border-stitch-danger bg-stitch-danger/10 ring-1 ring-stitch-danger'
                      : 'border-stitch-border hover:bg-stitch-higher'
                  }`}
                  title="Show this link in the matrix"
                  {...selectable(
                    selectedFinding?.row === c.row && selectedFinding.col === c.col,
                    () => onSelectFinding(c.row, c.col),
                  )}
                >
                  {reqCode(source.id)} (approved) depends on {reqCode(target.id)}, edited after that approval.
                  {hint}
                </li>
              );
            })}
          </ul>
        )}
      </section>

      <section>
        <h3 className={sectionTitle}>Legend</h3>
        <div className="grid grid-cols-2 gap-x-3 gap-y-1.5">
          {DSM_LINK_TYPES.map((t) => {
            const meta = linkTypeMeta(t);
            return (
              <div key={t} className="flex items-center gap-2">
                <span className={`inline-flex h-[18px] w-[18px] items-center justify-center rounded border border-stitch-border font-mono text-[10px] font-bold ${meta.text}`}>
                  {meta.letter}
                </span>
                {meta.label}
              </div>
            );
          })}
          <div className="flex items-center gap-2">
            <span className="h-[18px] w-[18px] rounded bg-stitch-muted" /> Diagonal
          </div>
          <div className="flex items-center gap-2">
            <span className="h-[18px] w-[18px] rounded border-2 border-stitch-accent/50" /> Group
          </div>
          <div className="flex items-center gap-2">
            <span className="h-[18px] w-[18px] rounded bg-amber-400/20" /> In a loop
          </div>
          <div className="flex items-center gap-2">
            <span className="h-[18px] w-[18px] rounded shadow-[inset_0_0_0_2px_var(--color-stitch-danger)]" />
            Upstream changed
          </div>
          <div className="flex items-center gap-2">
            <span className="h-2 w-2 rounded-full bg-green-600" /> Approved
          </div>
          <div className="flex items-center gap-2">
            <span className="h-2 w-2 rounded-full bg-amber-500" /> Reviewed
          </div>
          <div className="flex items-center gap-2">
            <span className="h-2 w-2 rounded-full border border-stitch-muted" /> Draft
          </div>
        </div>
      </section>
    </aside>
  );
}
