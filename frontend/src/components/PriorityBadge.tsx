/** Priority chip aligned with Image 2.html mock (P1 / P2 / P3). */
export default function PriorityBadge({ value }: { value: string }) {
  const v = value.trim().toUpperCase();
  const isP1 = /^(P1|1|HIGH)/i.test(v) || v.includes('P1');
  const isP3 = /^(P3|3|LOW)/i.test(v) || v.includes('P3');
  const isP2 = /^(P2|2|MED)/i.test(v) || v.includes('P2');

  if (isP1) {
    return (
      <div className="inline-flex items-center gap-1 text-red-200 font-bold text-[10px] bg-red-500/20 px-2 py-1 rounded border border-red-500/25">
        <span className="material-symbols-outlined text-xs">priority_high</span>
        {v === '—' ? 'P1' : value}
      </div>
    );
  }
  if (isP3) {
    return (
      <div className="inline-flex items-center gap-1 text-stitch-accent font-bold text-[10px] bg-stitch-accent/15 px-2 py-1 rounded border border-stitch-accent/25">
        <span className="material-symbols-outlined text-xs">low_priority</span>
        {v === '—' ? 'P3' : value}
      </div>
    );
  }
  if (isP2) {
    return (
      <div className="inline-flex items-center gap-1 text-stitch-accent font-bold text-[10px] bg-stitch-accent/12 px-2 py-1 rounded border border-stitch-accent/20">
        <span className="material-symbols-outlined text-xs">keyboard_arrow_up</span>
        {v === '—' ? 'P2' : value}
      </div>
    );
  }
  return (
    <span className="text-[10px] font-semibold text-stitch-muted px-2 py-1 rounded bg-white/[0.06]">
      {value}
    </span>
  );
}
