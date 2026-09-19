interface CsvDownloadButtonProps {
  onClick: () => void;
}

export function CsvDownloadButton({ onClick }: CsvDownloadButtonProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      title="Download CSV"
      className="p-2 text-stitch-muted hover:text-stitch-accent transition-colors"
    >
      <span className="material-symbols-outlined">file_download</span>
    </button>
  );
}

interface ExcelDownloadButtonProps {
  onClick: () => void;
  busy?: boolean;
}

export function ExcelDownloadButton({ onClick, busy = false }: ExcelDownloadButtonProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={busy}
      title="Download Excel (whole project)"
      aria-label="Download Excel"
      className="p-2 text-stitch-muted hover:text-stitch-accent transition-colors disabled:opacity-50"
    >
      <span className={`material-symbols-outlined${busy ? ' animate-spin' : ''}`}>
        {busy ? 'progress_activity' : 'table_chart'}
      </span>
    </button>
  );
}
