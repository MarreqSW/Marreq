/** Dark-theme status chips (Stitch + RVM mock: APPROVED solid, VERIFIED warm, etc.). */
function classesForTitle(title: string): string {
  const t = title.toLowerCase();
  if (t.includes('approved')) {
    return 'bg-stitch-accent-dim text-white border-transparent';
  }
  if (t.includes('verified') || t.includes('accepted')) {
    return 'bg-amber-500/20 text-amber-200 border-amber-500/30';
  }
  if (t.includes('fail') || t.includes('reject')) {
    return 'bg-red-500/15 text-red-300 border-red-500/25';
  }
  if (t.includes('review') || t.includes('pending')) {
    return 'bg-amber-500/15 text-amber-200 border-amber-500/25';
  }
  if (t.includes('draft')) {
    return 'bg-white/8 text-stitch-muted border-stitch-border';
  }
  return 'bg-white/8 text-stitch-muted border-stitch-border';
}

export function StatusBadge({ title }: { title: string }) {
  const cls = classesForTitle(title);
  return (
    <span className={`px-2 py-0.5 rounded-md text-[11px] font-semibold border ${cls}`}>
      {title}
    </span>
  );
}
