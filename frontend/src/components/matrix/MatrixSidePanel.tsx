import type { MatrixLink } from '@/api/types';
import { StatusBadge } from '@/components/StatusBadge';
import { STATUS_GROUP_OPTIONS } from '@/lib/verificationStatusSemantic';

export interface SuspectItem {
  row: number;
  col: number;
  reqCode: string;
  verCode: string;
  link: MatrixLink;
}

export interface GapItem {
  index: number;
  code: string;
  title: string;
}

type Props = {
  suspects: SuspectItem[];
  reqsWithoutVerification: GapItem[];
  versWithoutRequirement: GapItem[];
  rollup: [string, { n: number; tagColor: string | null }][];
  selected:
    | { kind: 'suspect'; row: number; col: number }
    | { kind: 'row'; row: number }
    | { kind: 'column'; col: number }
    | null;
  onSelectSuspect: (row: number, col: number) => void;
  onSelectRow: (row: number) => void;
  onSelectColumn: (col: number) => void;
  onReview: (link: MatrixLink) => void;
  onClear: (link: MatrixLink) => void;
  /** `${req_id}-${verification_id}` of the link whose review/clear is running. */
  busyKey: string | null;
  canClear: boolean;
};

const sectionTitle =
  'flex justify-between text-[10px] uppercase tracking-widest text-stitch-muted font-bold mb-2';
const GAP_LIMIT = 8;

export default function MatrixSidePanel({
  suspects,
  reqsWithoutVerification,
  versWithoutRequirement,
  rollup,
  selected,
  onSelectSuspect,
  onSelectRow,
  onSelectColumn,
  onReview,
  onClear,
  busyKey,
  canClear,
}: Props) {
  const code = (text: string) => <span className="font-mono text-[11px] font-semibold text-stitch-accent">{text}</span>;
  const action =
    'rounded border border-stitch-border px-2 py-0.5 text-[10px] font-bold uppercase tracking-wider text-stitch-accent hover:bg-stitch-higher disabled:opacity-40';

  return (
    <aside className="rounded-xl border border-stitch-border bg-stitch-surface p-4 space-y-5 text-xs" aria-label="Matrix details">
      <section>
        <h3 className={sectionTitle}>
          Suspect links <span className="font-mono text-stitch-fg">{suspects.length}</span>
        </h3>
        {suspects.length === 0 ? (
          <p className="text-stitch-muted">No suspect links for the current filters.</p>
        ) : (
          <ul className="space-y-2">
            {suspects.map((s) => {
              const key = `${s.link.req_id}-${s.link.verification_id}`;
              const isSelected = selected?.kind === 'suspect' && selected.row === s.row && selected.col === s.col;
              return (
                <li
                  key={key}
                  data-testid={`matrix-suspect-${s.row}-${s.col}`}
                  className={`rounded-lg border p-2.5 ${
                    isSelected ? 'border-stitch-danger bg-stitch-danger/10 ring-1 ring-stitch-danger' : 'border-stitch-border'
                  }`}
                >
                  <button
                    type="button"
                    aria-pressed={isSelected}
                    onClick={() => onSelectSuspect(s.row, s.col)}
                    className="block w-full text-left hover:opacity-80"
                    title="Show this link in the matrix"
                  >
                    {code(s.reqCode)} <span className="text-stitch-muted">→</span> {code(s.verCode)}
                    <span className="mt-0.5 block text-stitch-muted">
                      {s.link.suspect_reason ?? 'Requirement changed after verification.'}
                    </span>
                  </button>
                  <div className="mt-2 flex gap-2">
                    <button type="button" className={action} disabled={busyKey === key} onClick={() => onReview(s.link)}>
                      {busyKey === key ? '…' : 'Review'}
                    </button>
                    <button
                      type="button"
                      className={action}
                      disabled={busyKey === key || !canClear}
                      onClick={() => onClear(s.link)}
                    >
                      Clear
                    </button>
                  </div>
                </li>
              );
            })}
          </ul>
        )}
      </section>

      <section>
        <h3 className={sectionTitle}>
          Coverage gaps{' '}
          <span className="font-mono text-stitch-fg">{reqsWithoutVerification.length + versWithoutRequirement.length}</span>
        </h3>
        <GapList
          title="Requirements without verification"
          items={reqsWithoutVerification}
          isSelected={(g) => selected?.kind === 'row' && selected.row === g.index}
          onSelect={(g) => onSelectRow(g.index)}
          testId="matrix-gap-row"
        />
        <GapList
          title="Verifications without requirement"
          items={versWithoutRequirement}
          isSelected={(g) => selected?.kind === 'column' && selected.col === g.index}
          onSelect={(g) => onSelectColumn(g.index)}
          testId="matrix-gap-col"
        />
      </section>

      {rollup.length > 0 ? (
        <section>
          <h3 className={sectionTitle}>Linked cells by status</h3>
          <div className="flex flex-wrap gap-2">
            {rollup.map(([label, { n, tagColor }]) => (
              <span key={label} className="inline-flex items-center gap-1.5">
                <StatusBadge title={label} tagColor={tagColor} />
                <span className="text-xs text-stitch-muted tabular-nums">×{n}</span>
              </span>
            ))}
          </div>
        </section>
      ) : null}

      <section>
        <h3 className={sectionTitle}>Legend</h3>
        <div className="grid grid-cols-2 gap-x-3 gap-y-1.5">
          {STATUS_GROUP_OPTIONS.map((g) => (
            <div key={g.id} className="flex items-center gap-2">
              <span className="inline-flex h-[18px] w-[18px] items-center justify-center rounded border border-stitch-border font-semibold">
                {g.symbol}
              </span>
              {g.label}
            </div>
          ))}
          <div className="flex items-center gap-2">
            <span className="h-[18px] w-[18px] rounded shadow-[inset_0_0_0_2px_var(--color-stitch-danger)]" /> Suspect link
          </div>
          <div className="flex items-center gap-2">
            <span className="h-[18px] w-[18px] border-t-2 border-stitch-accent/40" /> Category
          </div>
          <div className="flex items-center gap-2">
            <span className="h-2 w-2 rounded-full bg-green-600" /> Approved
          </div>
          <div className="flex items-center gap-2">
            <span className="h-2 w-2 rounded-full bg-amber-500" /> Reviewed
          </div>
        </div>
      </section>
    </aside>
  );
}

function GapList({
  title,
  items,
  isSelected,
  onSelect,
  testId,
}: {
  title: string;
  items: GapItem[];
  isSelected: (g: GapItem) => boolean;
  onSelect: (g: GapItem) => void;
  testId: string;
}) {
  if (items.length === 0) {
    return <p className="mb-2 text-stitch-muted">{title}: none.</p>;
  }
  return (
    <div className="mb-3">
      <p className="mb-1 font-semibold text-stitch-fg">
        {title} ({items.length})
      </p>
      <ul className="flex flex-wrap gap-1.5">
        {items.slice(0, GAP_LIMIT).map((g) => (
          <li key={g.index}>
            <button
              type="button"
              data-testid={`${testId}-${g.index}`}
              aria-pressed={isSelected(g)}
              title={`${g.title} — show in the matrix`}
              onClick={() => onSelect(g)}
              className={`rounded border px-1.5 py-0.5 font-mono text-[10.5px] font-semibold text-stitch-accent ${
                isSelected(g) ? 'border-amber-500 bg-amber-400/20' : 'border-stitch-border hover:bg-stitch-higher'
              }`}
            >
              {g.code}
            </button>
          </li>
        ))}
        {items.length > GAP_LIMIT ? (
          <li className="self-center text-stitch-muted">+{items.length - GAP_LIMIT} more</li>
        ) : null}
      </ul>
    </div>
  );
}
