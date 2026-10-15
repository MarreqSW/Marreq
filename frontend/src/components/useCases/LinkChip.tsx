import type { UseCaseLink } from '@/api/types';

/** A requirement or verification linked to a use case element (screen 7 of #388). */
export default function LinkChip({ link, onOpen }: { link: UseCaseLink; onOpen?: (link: UseCaseLink) => void }) {
  const title = link.suspect
    ? `${link.target_reference_code} changed since it was linked${link.suspect_reason ? ` (${link.suspect_reason})` : ''}`
    : `${link.target_reference_code}: ${link.target_title}`;
  return (
    <button
      type="button"
      title={title}
      onClick={() => onOpen?.(link)}
      className={`inline-flex items-center gap-1 rounded-full border px-2 py-0.5 font-mono text-[10px] font-semibold whitespace-nowrap ${
        link.suspect
          ? 'border-amber-500 bg-amber-500/10 text-amber-800 dark:text-amber-300'
          : 'border-stitch-accent/30 bg-stitch-accent/5 text-stitch-accent'
      }`}
    >
      <span className="material-symbols-outlined text-[12px]" aria-hidden>
        {link.suspect ? 'link_off' : link.target_type === 'verification' ? 'verified' : 'link'}
      </span>
      {link.target_reference_code}
    </button>
  );
}
