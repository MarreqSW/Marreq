/** Semantic verification status buckets (aligned with matrix symbol legend). */
export type StatusSemanticGroup =
  | 'pass'
  | 'verified'
  | 'pending'
  | 'draft'
  | 'fail'
  | 'other';

export const STATUS_GROUP_OPTIONS: ReadonlyArray<{
  id: StatusSemanticGroup;
  label: string;
  symbol: string;
  symbolClass: string;
}> = [
  { id: 'pass', label: 'Pass / complete', symbol: '✓', symbolClass: 'text-emerald-400' },
  { id: 'verified', label: 'Verified / accepted', symbol: '✓', symbolClass: 'text-amber-300' },
  { id: 'pending', label: 'Pending / review', symbol: '◐', symbolClass: 'text-amber-200' },
  { id: 'draft', label: 'Draft', symbol: '○', symbolClass: 'text-stitch-muted' },
  { id: 'fail', label: 'Fail / reject', symbol: '✗', symbolClass: 'text-red-300' },
  { id: 'other', label: 'Other', symbol: '●', symbolClass: 'text-stitch-muted' },
];

/** Classify a catalog status title into a semantic group (same precedence as cell glyphs). */
export function statusSemanticGroup(
  statusTitle: string,
  tagColor?: string | null,
): StatusSemanticGroup {
  const t = statusTitle.toLowerCase();
  if (t.includes('fail') || t.includes('reject')) return 'fail';
  if (
    /\bpass\b/.test(t) ||
    t.includes('passed') ||
    t.includes('success') ||
    t.includes('complete') ||
    t === 'ok'
  ) {
    return 'pass';
  }
  if (t.includes('verified') || t.includes('accepted')) return 'verified';
  if (
    t.includes('pending') ||
    t.includes('review') ||
    t.includes('progress') ||
    t.includes('blocked')
  ) {
    return 'pending';
  }
  if (t.includes('draft')) return 'draft';
  if (tagColor && /^#[0-9A-Fa-f]{6}$/.test(tagColor.trim())) return 'other';
  return 'other';
}

export const GLYPH_BY_GROUP: Record<
  StatusSemanticGroup,
  { symbol: string; className: string }
> = {
  fail: { symbol: '✗', className: 'text-red-300' },
  pass: { symbol: '✓', className: 'text-emerald-400' },
  verified: { symbol: '✓', className: 'text-amber-300' },
  pending: { symbol: '◐', className: 'text-amber-200' },
  draft: { symbol: '○', className: 'text-stitch-muted' },
  other: { symbol: '●', className: 'text-stitch-muted' },
};

/** Visual + tooltip for a verification status in a matrix cell (catalog titles vary by project). */
export function statusGlyph(
  statusTitle: string,
  tagColor: string | null | undefined,
): { symbol: string; className: string } {
  const group = statusSemanticGroup(statusTitle, tagColor);
  const glyph = GLYPH_BY_GROUP[group];
  if (group === 'other' && tagColor && /^#[0-9A-Fa-f]{6}$/.test(tagColor.trim())) {
    return { symbol: glyph.symbol, className: '' };
  }
  return glyph;
}
