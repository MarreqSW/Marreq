import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import type { UseCaseRef } from '@/api/types';

type Props = {
  basePath: string;
  /** Loads the use cases that link to the artifact. */
  load: () => Promise<UseCaseRef[]>;
  /** Rendered as a card of its own, or as a section inside another card. */
  variant?: 'card' | 'section';
};

/** Where a requirement or verification is used (screen 7 of #388, issue #400). */
export default function UsedInUseCasesCard({ basePath, load, variant = 'card' }: Props) {
  const [refs, setRefs] = useState<UseCaseRef[] | null>(null);

  useEffect(() => {
    let live = true;
    Promise.resolve()
      .then(load)
      .then((r) => live && setRefs(r))
      .catch(() => live && setRefs([]));
    return () => {
      live = false;
    };
  }, [load]);

  const list =
    refs == null ? (
      <p className="text-xs text-stitch-muted">Loading…</p>
    ) : refs.length === 0 ? (
      <p className="text-xs text-stitch-muted">None</p>
    ) : (
      <ul className="space-y-2">
        {refs.map((r) => (
          <li key={r.link_id}>
            <Link
              to={`${basePath}/use-cases/${r.use_case_id}`}
              className={`flex items-center gap-2 rounded-lg border bg-stitch-elevated p-3 transition-colors hover:bg-stitch-higher ${
                r.suspect ? 'border-amber-500' : 'border-stitch-border'
              }`}
            >
              <div className="min-w-0 flex-1">
                <span className="block font-mono text-[10px] font-bold text-stitch-accent">
                  {r.reference_code}
                  {r.element_label ? (
                    <span className="font-sans font-normal text-stitch-muted">
                      {' '}
                      · {/^\d/.test(r.element_label) ? `step ${r.element_label}` : r.element_label}
                    </span>
                  ) : null}
                </span>
                <span className="block truncate text-xs text-stitch-fg">{r.title}</span>
              </div>
              {r.suspect ? (
                <span
                  title="Changed since linked"
                  className="material-symbols-outlined text-base text-amber-600"
                  aria-label="Suspect link"
                >
                  link_off
                </span>
              ) : null}
            </Link>
          </li>
        ))}
      </ul>
    );

  if (variant === 'section') {
    return (
      <div>
        <p className="mb-2 text-[10px] font-bold uppercase tracking-wider text-stitch-muted">Used in use cases</p>
        {list}
      </div>
    );
  }
  return (
    <section className="mt-8 overflow-hidden rounded-xl border border-stitch-border bg-stitch-surface shadow-stitch">
      <div className="flex items-center gap-2 border-b border-stitch-border bg-stitch-elevated px-6 py-3">
        <span className="material-symbols-outlined text-xl text-stitch-accent">route</span>
        <h2 className="font-headline text-sm font-bold text-stitch-accent">Used in use cases</h2>
      </div>
      <div className="px-6 py-4">{list}</div>
    </section>
  );
}
