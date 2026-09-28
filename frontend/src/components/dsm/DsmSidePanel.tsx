import { Link } from 'react-router-dom';
import type { Dsm } from '@/api/types';
import { DSM_LINK_TYPES, linkTypeMeta } from '@/utils/dsm';

type Props = {
  dsm: Dsm;
  basePath: string;
  highlightedLoop: number | null;
  onHighlightLoop: (loopIndex: number | null) => void;
};

const sectionTitle =
  'flex justify-between text-[10px] uppercase tracking-widest text-stitch-muted font-bold mb-2';

export default function DsmSidePanel({ dsm, basePath, highlightedLoop, onHighlightLoop }: Props) {
  const byId = new Map(dsm.requirements.map((r) => [r.id, r]));
  const code = (id: number) => byId.get(id)?.reference_code ?? `#${id}`;
  const changed = dsm.cells.filter((c) => c.upstream_changed);

  const reqLink = (id: number) => (
    <Link
      key={id}
      to={`${basePath}/requirements/${id}`}
      className="font-mono text-[11px] font-semibold text-stitch-accent hover:underline"
    >
      {code(id)}
    </Link>
  );

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
                className={`rounded-lg border p-2.5 ${
                  highlightedLoop === i ? 'border-amber-500 bg-amber-400/10' : 'border-stitch-border'
                }`}
                onMouseEnter={() => onHighlightLoop(i)}
                onMouseLeave={() => onHighlightLoop(null)}
              >
                <span className="inline-block rounded bg-amber-400/20 px-1.5 py-0.5 font-mono text-[10px] font-bold text-amber-700 dark:text-amber-300">
                  LOOP {i + 1} · {l.requirement_ids.length} requirements
                </span>
                <p className="mt-1.5 leading-relaxed">
                  {[...l.path, l.path[0]!].map((id, k) => (
                    <span key={`${id}-${k}`}>
                      {k > 0 ? <span className="text-stitch-muted"> → </span> : null}
                      {reqLink(id)}
                    </span>
                  ))}
                </p>
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
                <li key={`${c.row}:${c.col}`} className="rounded-lg border border-stitch-border p-2.5">
                  {reqLink(source.id)} (approved) depends on {reqLink(target.id)}, edited after that approval.
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
