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
