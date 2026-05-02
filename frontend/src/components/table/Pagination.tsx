import { paginationItems } from '@/utils/tableUtils';

interface PaginationProps {
  page: number;
  pageCount: number;
  onPageChange: (page: number) => void;
}

export function Pagination({ page, pageCount, onPageChange }: PaginationProps) {
  return (
    <div className="flex items-center gap-1">
      <button
        type="button"
        disabled={page <= 1}
        onClick={() => onPageChange(Math.max(1, page - 1))}
        className="p-1 text-stitch-muted hover:text-stitch-accent transition-colors disabled:opacity-30"
      >
        <span className="material-symbols-outlined">chevron_left</span>
      </button>
      <div className="flex gap-1 items-center">
        {paginationItems(page, pageCount).map((item, idx) =>
          item === 'dots' ? (
            <span key={`dots-${idx}`} className="px-1 text-stitch-muted text-xs">
              …
            </span>
          ) : (
            <button
              key={item}
              type="button"
              onClick={() => onPageChange(item)}
              className={`w-8 h-8 flex items-center justify-center rounded text-xs font-bold transition-colors ${
                item === page
                  ? 'bg-stitch-accent text-stitch-canvas'
                  : 'hover:bg-stitch-elevated text-stitch-fg'
              }`}
            >
              {item}
            </button>
          ),
        )}
      </div>
      <button
        type="button"
        disabled={page >= pageCount}
        onClick={() => onPageChange(Math.min(pageCount, page + 1))}
        className="p-1 text-stitch-muted hover:text-stitch-accent transition-colors disabled:opacity-30"
      >
        <span className="material-symbols-outlined">chevron_right</span>
      </button>
    </div>
  );
}
