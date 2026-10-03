/** Semantic verification status buckets (aligned with matrix symbol legend). */
export type StatusSemanticGroup =
  | 'pass'
  | 'verified'
  | 'pending'
  | 'draft'
  | 'fail'
  | 'other';

/**
 * Text colour of each group's symbol, for both themes (issue #363). The light
 * shades keep at least 4.4:1 on white; the dark ones are unchanged.
 */
export const GROUP_TEXT_CLASS: Record<StatusSemanticGroup, string> = {
  pass: 'text-emerald-700 dark:text-emerald-400',
  verified: 'text-amber-800 dark:text-amber-300',
  pending: 'text-amber-700 dark:text-amber-200',
  draft: 'text-stitch-muted',
  fail: 'text-red-700 dark:text-red-300',
  other: 'text-stitch-muted',
};

export const STATUS_GROUP_OPTIONS: ReadonlyArray<{
  id: StatusSemanticGroup;
  label: string;
  symbol: string;
  symbolClass: string;
}> = [
  { id: 'pass', label: 'Pass / complete', symbol: '✓', symbolClass: GROUP_TEXT_CLASS.pass },
  { id: 'verified', label: 'Verified / accepted', symbol: '✓', symbolClass: GROUP_TEXT_CLASS.verified },
  { id: 'pending', label: 'Pending / review', symbol: '◐', symbolClass: GROUP_TEXT_CLASS.pending },
  { id: 'draft', label: 'Draft', symbol: '○', symbolClass: GROUP_TEXT_CLASS.draft },
  { id: 'fail', label: 'Fail / reject', symbol: '✗', symbolClass: GROUP_TEXT_CLASS.fail },
  { id: 'other', label: 'Other', symbol: '●', symbolClass: GROUP_TEXT_CLASS.other },
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
  fail: { symbol: '✗', className: GROUP_TEXT_CLASS.fail },
  pass: { symbol: '✓', className: GROUP_TEXT_CLASS.pass },
  verified: { symbol: '✓', className: GROUP_TEXT_CLASS.verified },
  pending: { symbol: '◐', className: GROUP_TEXT_CLASS.pending },
  draft: { symbol: '○', className: GROUP_TEXT_CLASS.draft },
  other: { symbol: '●', className: GROUP_TEXT_CLASS.other },
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
