import { statusChipAppearance } from '@/components/StatusBadge';

type Props = {
  title: string;
  tagColor: string | null;
  /** e.g. "requirement status", for the accessible name. */
  category: string;
  onRemove: () => void;
};

/**
 * An active exact-status filter: one chip in the status's own colours (the
 * same as `StatusBadge`), with a remove mark. No second selection box around
 * it (issue #361).
 */
export default function StatusFilterChip({ title, tagColor, category, onRemove }: Props) {
  const { className, style } = statusChipAppearance(title, tagColor);
  return (
    <button
      type="button"
      onClick={onRemove}
      aria-label={`Remove ${category} filter: ${title}`}
      title={`Remove ${category} filter: ${title}`}
      className={`inline-flex items-center gap-1 rounded-md border pl-2 pr-1.5 py-0.5 text-[11px] font-semibold hover:brightness-95 dark:hover:brightness-110 focus:outline-hidden focus-visible:ring-2 focus-visible:ring-stitch-accent ${className}`.trim()}
      style={style}
    >
      {title}
      <span aria-hidden className="text-[13px] leading-none opacity-70">
        ×
      </span>
    </button>
  );
}
